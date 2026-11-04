//! Sends the cases to AppSync and writes the results as test vectors.

use std::fs;
use std::path::Path as FilePath;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::cases::{Case, Control};
use crate::classify::{Outcome, classify};
use crate::graphql::{Path, Scalar, descriptions_request, request};

pub trait Transport: Sync {
    /// Returns the HTTP status and body, or an error if no response arrived.
    fn post(&self, body: &str) -> Result<(u16, String), String>;
}

pub struct HttpTransport {
    agent: ureq::Agent,
    url: String,
    api_key: String,
}

impl HttpTransport {
    pub fn new(url: String, api_key: String) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        HttpTransport { agent, url, api_key }
    }
}

impl Transport for HttpTransport {
    fn post(&self, body: &str) -> Result<(u16, String), String> {
        let mut response = self
            .agent
            .post(&self.url)
            .header("content-type", "application/json")
            .header("x-api-key", &self.api_key)
            .send(body)
            .map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(|e| e.to_string())?;
        Ok((status, text))
    }
}

#[derive(Clone, Copy)]
pub struct Policy {
    pub workers: usize,
    pub retries: u32,
    pub base_delay: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            workers: 8,
            retries: 6,
            base_delay: Duration::from_millis(250),
        }
    }
}

/// Sends one request, retrying throttling and server errors. A request that keeps failing is
/// an error rather than a response, so it can never be recorded as a verdict on the value.
pub fn send(transport: &dyn Transport, body: &str, policy: Policy) -> Result<(u16, String), String> {
    let mut delay = policy.base_delay;
    let mut last_failure = String::new();
    for attempt in 0..=policy.retries {
        if attempt > 0 {
            thread::sleep(delay);
            delay *= 2;
        }
        match transport.post(body) {
            Ok((status @ (401 | 403), text)) => {
                return Err(format!("AppSync refused the request (HTTP {status}): {text}"));
            }
            Ok((status @ (429 | 500..=599), text)) => {
                last_failure = format!("HTTP {status}: {text}");
            }
            Ok(response) => return Ok(response),
            Err(error) => last_failure = error,
        }
    }
    Err(format!("giving up after {} attempts: {last_failure}", policy.retries + 1))
}

/// Sends every body, returning responses in the same order.
pub fn send_all(
    transport: &dyn Transport,
    bodies: &[String],
    policy: Policy,
) -> Result<Vec<(u16, String)>, String> {
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let responses: Mutex<Vec<Option<(u16, String)>>> = Mutex::new(vec![None; bodies.len()]);
    let failure: Mutex<Option<String>> = Mutex::new(None);

    thread::scope(|scope| {
        for _ in 0..policy.workers.max(1) {
            scope.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(body) = bodies.get(index) else { break };
                    match send(transport, body, policy) {
                        Ok(response) => responses.lock().unwrap()[index] = Some(response),
                        Err(error) => {
                            stop.store(true, Ordering::Relaxed);
                            failure.lock().unwrap().get_or_insert(error);
                        }
                    }
                    let finished = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if finished.is_multiple_of(500) {
                        eprintln!("  {finished}/{} requests", bodies.len());
                    }
                }
            });
        }
    });

    if let Some(error) = failure.into_inner().unwrap() {
        return Err(error);
    }
    Ok(responses
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|response| response.expect("every request finished"))
        .collect())
}

/// Confirms the API behaves as expected before any results are trusted.
pub fn preflight(transport: &dyn Transport, controls: &[Control], policy: Policy) -> Result<(), String> {
    for control in controls {
        let name = control.scalar.type_name();
        for path in Path::ALL {
            let good = request(control.scalar, path, &control.good);
            let (status, body) = send(transport, &good, policy)?;
            match classify(path, status, &body) {
                Outcome::Accepted { value } if value == control.good => {}
                other => {
                    return Err(format!(
                        "{name} {path:?} should echo {} but got {other:?}",
                        control.good
                    ));
                }
            }

            let bad = request(control.scalar, path, &control.bad);
            let (status, body) = send(transport, &bad, policy)?;
            let outcome = classify(path, status, &body);
            if !outcome.is_rejected() {
                return Err(format!(
                    "{name} {path:?} should reject {} but got {outcome:?}",
                    control.bad
                ));
            }
        }
    }
    Ok(())
}

/// Results for one case, in the same order as `Path::ALL`.
pub struct CaseResult {
    pub case: Case,
    pub outcomes: [Outcome; 3],
}

/// Runs every case on every path `passes` times and fails if any verdict is unstable or unclear.
pub fn run(
    transport: &dyn Transport,
    cases: &[Case],
    passes: usize,
    policy: Policy,
) -> Result<Vec<CaseResult>, String> {
    let tasks: Vec<(usize, Path)> = (0..cases.len())
        .flat_map(|index| Path::ALL.map(|path| (index, path)))
        .collect();
    let bodies: Vec<String> = tasks
        .iter()
        .map(|&(index, path)| request(cases[index].scalar, path, &cases[index].value))
        .collect();

    let mut first: Vec<Outcome> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for pass in 0..passes.max(1) {
        eprintln!("pass {} of {}: {} requests", pass + 1, passes.max(1), bodies.len());
        let responses = send_all(transport, &bodies, policy)?;
        let outcomes: Vec<Outcome> = tasks
            .iter()
            .zip(&responses)
            .map(|(&(_, path), (status, body))| classify(path, *status, body))
            .collect();
        if pass == 0 {
            first = outcomes;
            continue;
        }
        for (i, outcome) in outcomes.iter().enumerate() {
            if !outcome.same_verdict(&first[i]) {
                let (index, path) = tasks[i];
                problems.push(format!(
                    "unstable: {} {} on {path:?}: {:?} then {outcome:?}",
                    cases[index].scalar.type_name(),
                    cases[index].value,
                    first[i],
                ));
            }
        }
    }

    for (i, outcome) in first.iter().enumerate() {
        if outcome.is_unexpected() {
            let (index, path) = tasks[i];
            problems.push(format!(
                "unexpected: {} {} on {path:?}: {outcome:?}",
                cases[index].scalar.type_name(),
                cases[index].value,
            ));
        }
    }
    if !problems.is_empty() {
        let total = problems.len();
        problems.truncate(25);
        return Err(format!("{total} problems, nothing written:\n{}", problems.join("\n")));
    }

    let mut outcomes = first.into_iter();
    Ok(cases
        .iter()
        .map(|case| CaseResult {
            case: case.clone(),
            outcomes: std::array::from_fn(|_| outcomes.next().expect("three outcomes per case")),
        })
        .collect())
}

#[derive(Serialize)]
struct VectorFile<'a> {
    format_version: u32,
    scalar: &'static str,
    recorded_at: &'a str,
    region: &'a str,
    cases: Vec<VectorCase<'a>>,
}

#[derive(Serialize)]
struct VectorCase<'a> {
    group: &'static str,
    value: &'a Value,
    input_variable: &'a Outcome,
    input_literal: &'a Outcome,
    output: &'a Outcome,
}

/// Writes one file per scalar. Files hold no account ID, API URL or key, so they can be published.
pub fn write_vectors(
    directory: &FilePath,
    results: &[CaseResult],
    recorded_at: &str,
    region: &str,
) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    for scalar in Scalar::ALL {
        let cases = results
            .iter()
            .filter(|result| result.case.scalar == scalar)
            .map(|result| VectorCase {
                group: result.case.group,
                value: &result.case.value,
                input_variable: &result.outcomes[0],
                input_literal: &result.outcomes[1],
                output: &result.outcomes[2],
            })
            .collect();
        let file = VectorFile {
            format_version: 1,
            scalar: scalar.type_name(),
            recorded_at,
            region,
            cases,
        };
        write_json(&directory.join(format!("{}.json", scalar.file_stem())), &file)?;
    }
    Ok(())
}

/// Saves AppSync's own description of each scalar, as returned by introspection.
pub fn write_descriptions(
    transport: &dyn Transport,
    directory: &FilePath,
    policy: Policy,
) -> Result<(), String> {
    let (status, body) = send(transport, &descriptions_request(), policy)?;
    let parsed: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let data = parsed.get("data").filter(|_| status == 200 && parsed.get("errors").is_none());
    let Some(data) = data else {
        return Err(format!("introspection failed (HTTP {status}): {body}"));
    };
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    write_json(&directory.join("scalar-descriptions.json"), data)
}

fn write_json<T: Serialize>(path: &FilePath, value: &T) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    text.push('\n');
    fs::write(path, text).map_err(|e| e.to_string())
}

/// Prints verdict counts per scalar, including cases where the paths disagree.
pub fn print_summary(results: &[CaseResult]) {
    println!("{:<14}{:>7}{:>10}{:>10}{:>9}{:>10}", "scalar", "cases", "accepted", "rejected", "mixed", "syntax");
    for scalar in Scalar::ALL {
        let of_scalar: Vec<&CaseResult> = results.iter().filter(|r| r.case.scalar == scalar).collect();
        let all = |test: fn(&Outcome) -> bool| {
            of_scalar.iter().filter(|r| r.outcomes.iter().all(test)).count()
        };
        let accepted = all(Outcome::is_accepted);
        let rejected = all(Outcome::is_rejected);
        let syntax = of_scalar
            .iter()
            .filter(|r| r.outcomes.iter().any(|o| matches!(o, Outcome::SyntaxError { .. })))
            .count();
        let mixed = of_scalar.len() - accepted - rejected - syntax;
        println!(
            "{:<14}{:>7}{:>10}{:>10}{:>9}{:>10}",
            scalar.type_name(),
            of_scalar.len(),
            accepted,
            rejected,
            mixed,
            syntax
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cases::{all, controls};
    use serde_json::json;
    use std::collections::HashMap;

    fn fast() -> Policy {
        Policy {
            workers: 4,
            retries: 2,
            base_delay: Duration::from_millis(1),
        }
    }

    /// Plays back canned responses, one per call.
    struct Script(Mutex<Vec<Result<(u16, String), String>>>);

    impl Script {
        fn new(mut responses: Vec<Result<(u16, String), String>>) -> Self {
            responses.reverse();
            Script(Mutex::new(responses))
        }
        fn remaining(&self) -> usize {
            self.0.lock().unwrap().len()
        }
    }

    impl Transport for Script {
        fn post(&self, _body: &str) -> Result<(u16, String), String> {
            self.0.lock().unwrap().pop().expect("no more scripted responses")
        }
    }

    /// Behaves like the echo API, accepting only values that start with a digit.
    struct FakeAppSync {
        flaky_value: Option<String>,
        calls: AtomicUsize,
    }

    impl FakeAppSync {
        fn new() -> Self {
            FakeAppSync {
                flaky_value: None,
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl Transport for FakeAppSync {
        fn post(&self, body: &str) -> Result<(u16, String), String> {
            let call = self.calls.fetch_add(1, Ordering::Relaxed);
            let request: Value = serde_json::from_str(body).unwrap();
            let query = request["query"].as_str().unwrap();
            if query.contains("__type") {
                return Ok((200, json!({ "data": { "AWSDate": { "name": "AWSDate", "description": "d" } } }).to_string()));
            }
            let output = query.contains("output");
            let value: Value = if output {
                serde_json::from_str(request["variables"]["json"].as_str().unwrap()).unwrap()
            } else if let Some(value) = request["variables"].get("value") {
                value.clone()
            } else {
                let start = query.find("value: ").unwrap() + 7;
                let end = query.rfind(") }").unwrap();
                serde_json::from_str(&query[start..end]).unwrap_or(json!("rejected-literal"))
            };
            let text = value.to_string();
            let mut good = text.trim_matches('"').starts_with(|c: char| c.is_ascii_digit());
            if self.flaky_value.as_deref() == Some(text.as_str()) && call.is_multiple_of(2) {
                good = !good;
            }
            let response = match (good, output) {
                (true, true) => json!({ "data": { "r": value } }),
                (true, false) => json!({ "data": { "r": text } }),
                (false, true) => json!({ "data": { "r": null }, "errors": [{ "message": "Can't serialize value (/r)" }] }),
                (false, false) => json!({ "data": null, "errors": [{ "message": "Variable 'value' has an invalid value" }] }),
            };
            Ok((200, response.to_string()))
        }
    }

    fn sample_cases() -> Vec<Case> {
        vec![
            Case { scalar: Scalar::Date, group: "a", value: json!("2021-01-11") },
            Case { scalar: Scalar::Date, group: "a", value: json!("nope") },
            Case { scalar: Scalar::Time, group: "b", value: json!("12:30") },
        ]
    }

    #[test]
    fn send_retries_throttling_and_server_errors() {
        let script = Script::new(vec![
            Ok((429, "slow down".into())),
            Ok((503, "busy".into())),
            Ok((200, "fine".into())),
        ]);
        assert_eq!(send(&script, "{}", fast()).unwrap(), (200, "fine".to_string()));
        assert_eq!(script.remaining(), 0);
    }

    #[test]
    fn send_retries_network_errors() {
        let script = Script::new(vec![Err("reset".into()), Ok((200, "fine".into()))]);
        assert_eq!(send(&script, "{}", fast()).unwrap().1, "fine");
    }

    #[test]
    fn send_gives_up_instead_of_returning_a_failure_as_a_response() {
        let script = Script::new(vec![
            Ok((500, "a".into())),
            Ok((500, "b".into())),
            Ok((500, "c".into())),
        ]);
        let error = send(&script, "{}", fast()).unwrap_err();
        assert!(error.contains("giving up after 3 attempts"), "{error}");
    }

    #[test]
    fn send_does_not_retry_auth_failures() {
        let script = Script::new(vec![Ok((401, "no".into())), Ok((200, "never reached".into()))]);
        let error = send(&script, "{}", fast()).unwrap_err();
        assert!(error.contains("HTTP 401"), "{error}");
        assert_eq!(script.remaining(), 1);
    }

    #[test]
    fn send_all_keeps_request_order() {
        struct Echo;
        impl Transport for Echo {
            fn post(&self, body: &str) -> Result<(u16, String), String> {
                Ok((200, body.to_string()))
            }
        }
        let bodies: Vec<String> = (0..200).map(|i| i.to_string()).collect();
        let responses = send_all(&Echo, &bodies, fast()).unwrap();
        let echoed: Vec<String> = responses.into_iter().map(|(_, body)| body).collect();
        assert_eq!(echoed, bodies);
    }

    #[test]
    fn send_all_stops_on_a_fatal_error() {
        struct Denied;
        impl Transport for Denied {
            fn post(&self, _: &str) -> Result<(u16, String), String> {
                Ok((401, "no".into()))
            }
        }
        let bodies: Vec<String> = (0..50).map(|i| i.to_string()).collect();
        assert!(send_all(&Denied, &bodies, fast()).is_err());
    }

    #[test]
    fn run_records_one_outcome_per_path() {
        let results = run(&FakeAppSync::new(), &sample_cases(), 2, fast()).unwrap();
        assert_eq!(results.len(), 3);
        assert!(results[0].outcomes.iter().all(Outcome::is_accepted));
        assert!(results[1].outcomes.iter().all(Outcome::is_rejected));
    }

    #[test]
    fn run_refuses_unstable_results() {
        let cases = vec![Case { scalar: Scalar::Date, group: "a", value: json!("2021-01-11") }];
        let fake = FakeAppSync {
            flaky_value: Some("\"2021-01-11\"".to_string()),
            calls: AtomicUsize::new(0),
        };
        let policy = Policy { workers: 1, ..fast() };
        let error = run(&fake, &cases, 2, policy).err().expect("should be unstable");
        assert!(error.contains("unstable"), "{error}");
    }

    #[test]
    fn run_refuses_unexpected_responses() {
        struct Broken;
        impl Transport for Broken {
            fn post(&self, _: &str) -> Result<(u16, String), String> {
                Ok((200, json!({ "errors": [{ "message": "A code error occurred" }] }).to_string()))
            }
        }
        let error = run(&Broken, &sample_cases(), 1, fast()).err().expect("should fail");
        assert!(error.contains("nothing written"), "{error}");
        assert!(error.contains("A code error occurred"), "{error}");
    }

    #[test]
    fn preflight_passes_against_a_working_api() {
        preflight(&FakeAppSync::new(), &controls(), fast()).unwrap();
    }

    #[test]
    fn preflight_fails_when_everything_is_rejected() {
        struct RejectAll;
        impl Transport for RejectAll {
            fn post(&self, _: &str) -> Result<(u16, String), String> {
                Ok((200, json!({ "errors": [{ "message": "Variable 'value' is bad" }] }).to_string()))
            }
        }
        let error = preflight(&RejectAll, &controls(), fast()).unwrap_err();
        assert!(error.contains("should echo"), "{error}");
    }

    #[test]
    fn vectors_hold_no_connection_details_and_keep_exact_numbers() {
        let directory = std::env::temp_dir().join(format!("appsync-vectors-{}", std::process::id()));
        let big: Value = serde_json::from_str("9223372036854775808").unwrap();
        let results = vec![CaseResult {
            case: Case { scalar: Scalar::Timestamp, group: "integer-range", value: big },
            outcomes: [
                Outcome::Rejected { error: "e".into() },
                Outcome::Rejected { error: "e".into() },
                Outcome::Rejected { error: "e".into() },
            ],
        }];
        write_vectors(&directory, &results, "2026-11-04", "us-east-1").unwrap();
        let text = fs::read_to_string(directory.join("aws-timestamp.json")).unwrap();
        assert!(text.contains("9223372036854775808"));
        assert!(text.contains("\"region\": \"us-east-1\""));
        let lowered = text.to_lowercase();
        assert!(!lowered.contains("appsync-api") && !lowered.contains("api_key"));
        let keys: HashMap<String, Value> = serde_json::from_str(&text).unwrap();
        assert_eq!(keys["cases"][0]["input_variable"]["result"], "rejected");
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn every_real_case_produces_a_valid_request() {
        for case in all() {
            for path in Path::ALL {
                let body = request(case.scalar, path, &case.value);
                let parsed: Value = serde_json::from_str(&body).unwrap();
                assert!(parsed["query"].is_string());
            }
        }
    }
}
