mod cases;
mod classify;
mod graphql;
mod record;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use classify::classify;
use graphql::{Path, Scalar, request};
use record::{HttpTransport, Policy, send};
use serde_json::Value;

const USAGE: &str = "\
Records how a live AppSync API treats date and time scalar values.

Usage:
  appsync-conformance list
  appsync-conformance probe <AWSDate|AWSTime|AWSDateTime|AWSTimestamp> <json-value>
  appsync-conformance record --out <dir> --region <region> --recorded-at <yyyy-mm-dd> [--passes <n>]

probe and record read the API from APPSYNC_URL and APPSYNC_API_KEY.";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("list") => {
            list();
            Ok(())
        }
        Some("probe") => probe(&args[1..]),
        Some("record") => record_command(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn list() {
    let cases = cases::all();
    for case in &cases {
        println!("{}\t{}\t{}", case.scalar.type_name(), case.group, case.value);
    }
    eprintln!("{} cases", cases.len());
}

fn transport() -> Result<HttpTransport, String> {
    let read = |name: &str| env::var(name).map_err(|_| format!("{name} is not set"));
    Ok(HttpTransport::new(read("APPSYNC_URL")?, read("APPSYNC_API_KEY")?))
}

/// Shows the raw response for one value on every path, for debugging and exploring new cases.
fn probe(args: &[String]) -> Result<(), String> {
    let [name, raw] = args else {
        return Err(USAGE.to_string());
    };
    let scalar = Scalar::from_type_name(name).ok_or_else(|| format!("unknown scalar {name}"))?;
    let value: Value = serde_json::from_str(raw).map_err(|e| format!("not valid JSON: {e}"))?;
    let transport = transport()?;
    for path in Path::ALL {
        let (status, body) = send(&transport, &request(scalar, path, &value), Policy::default())?;
        println!("{path:?}\n  HTTP {status}: {body}\n  {:?}", classify(path, status, &body));
    }
    Ok(())
}

fn record_command(args: &[String]) -> Result<(), String> {
    let mut out = None;
    let mut region = None;
    let mut recorded_at = None;
    let mut passes = 2;
    let mut flags = args.iter();
    while let Some(flag) = flags.next() {
        let value = flags.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--out" => out = Some(PathBuf::from(value)),
            "--region" => region = Some(value.clone()),
            "--recorded-at" => recorded_at = Some(value.clone()),
            "--passes" => passes = value.parse().map_err(|_| format!("bad --passes value {value}"))?,
            other => return Err(format!("unknown flag {other}\n\n{USAGE}")),
        }
    }
    let out = out.ok_or("--out is required")?;
    let region = region.ok_or("--region is required")?;
    let recorded_at = recorded_at.ok_or("--recorded-at is required")?;

    let transport = transport()?;
    let policy = Policy::default();
    let cases = cases::all();

    eprintln!("checking control values");
    record::preflight(&transport, &cases::controls(), policy)?;

    let results = record::run(&transport, &cases, passes, policy)?;
    record::write_vectors(&out, &results, &recorded_at, &region)?;
    record::write_descriptions(&transport, &out, policy)?;
    record::print_summary(&results);
    eprintln!("wrote vectors to {}", out.display());
    Ok(())
}
