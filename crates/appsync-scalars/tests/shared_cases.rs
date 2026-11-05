//! Runs the test data shared by every language, including the recorded live results.

use std::fs;
use std::path::{Path, PathBuf};

use appsync_scalars::{Scalar, is_valid};
use serde_json::Value;

fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn load(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

fn scalar(name: &str) -> Scalar {
    [Scalar::Date, Scalar::Time, Scalar::DateTime, Scalar::Timestamp]
        .into_iter()
        .find(|s| s.name() == name)
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

#[test]
fn smoke_cases() {
    let cases = load(&conformance_dir().join("smoke.json"));
    let cases = cases.as_array().unwrap();
    assert!(!cases.is_empty(), "smoke.json has no cases");
    for case in cases {
        let scalar = scalar(case["scalar"].as_str().unwrap());
        let text = as_text(scalar, &case["value"]).expect("smoke value has a text form");
        let expected = case["valid"].as_bool().unwrap();
        assert_eq!(is_valid(scalar, &text), expected, "{} {text:?}", scalar.name());
    }
}

/// Compares against what AppSync accepted as a variable and as resolver output. Inline literals
/// are left out because AppSync wraps oversized integers there: 2^64 comes through as 0.
#[test]
fn live_vectors() {
    let directory = conformance_dir().join("vectors");
    let stems = ["aws-date", "aws-time", "aws-datetime", "aws-timestamp"];
    let files: Vec<PathBuf> = stems.iter().map(|stem| directory.join(format!("{stem}.json"))).collect();

    let mut mismatches = Vec::new();
    let mut total = 0;
    for file in files {
        let vectors = load(&file);
        let scalar = scalar(vectors["scalar"].as_str().unwrap());
        let cases = vectors["cases"].as_array().unwrap();
        assert!(!cases.is_empty(), "{} has no recorded cases", file.display());
        for case in cases {
            let Some(text) = as_text(scalar, &case["value"]) else { continue };
            total += 1;
            for path in ["input_variable", "output"] {
                let accepted = case[path]["result"] == "accepted";
                if is_valid(scalar, &text) != accepted {
                    mismatches.push(format!(
                        "{} {text:?} [{}] {path}: AppSync accepted = {accepted}",
                        scalar.name(),
                        case["group"].as_str().unwrap(),
                    ));
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} of {total} live results disagree:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
