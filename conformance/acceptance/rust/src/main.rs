//! Checks an installed appsync-scalars against the shared cases and recorded AppSync results.
//!
//! Usage: appsync-scalars-acceptance <conformance directory>

use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

use appsync_scalars::{Scalar, is_valid};
use serde_json::Value;

fn load(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

fn scalar(name: &str) -> Scalar {
    [Scalar::Date, Scalar::Time, Scalar::DateTime, Scalar::Timestamp]
        .into_iter()
        .find(|scalar| scalar.name() == name)
        .unwrap_or_else(|| panic!("unknown scalar {name}"))
}

/// The form a value takes in the Rust API, or `None` if the API has no way to express it.
fn as_text(scalar: Scalar, value: &Value) -> Option<String> {
    match (scalar, value) {
        (Scalar::Timestamp, Value::Number(number)) => Some(number.to_string()),
        (Scalar::Timestamp, _) => None,
        (_, Value::String(text)) => Some(text.clone()),
        _ => None,
    }
}

fn main() -> ExitCode {
    let directory = env::args().nth(1).expect("usage: <conformance directory>");
    let conformance = Path::new(&directory);

    let mut expected = Vec::new();
    for case in load(&conformance.join("smoke.json")).as_array().unwrap() {
        let scalar = scalar(case["scalar"].as_str().unwrap());
        let text = as_text(scalar, &case["value"]).expect("shared case has a text form");
        expected.push((scalar, text, case["valid"].as_bool().unwrap()));
    }
    for stem in ["aws-date", "aws-time", "aws-datetime", "aws-timestamp"] {
        let vectors = load(&conformance.join("vectors").join(format!("{stem}.json")));
        let scalar = scalar(vectors["scalar"].as_str().unwrap());
        let cases = vectors["cases"].as_array().unwrap();
        assert!(!cases.is_empty(), "{stem}.json has no recorded cases");
        for case in cases {
            let Some(text) = as_text(scalar, &case["value"]) else { continue };
            for path in ["input_variable", "output"] {
                expected.push((scalar, text.clone(), case[path]["result"] == "accepted"));
            }
        }
    }

    assert!(!expected.is_empty(), "no cases to check");
    let mut matched = 0;
    for (scalar, text, valid) in &expected {
        if is_valid(*scalar, text) == *valid {
            matched += 1;
        } else {
            println!("{} {text:?}: expected valid = {valid}", scalar.name());
        }
    }
    println!("Rust: {matched} of {} checks match", expected.len());
    if matched == expected.len() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
