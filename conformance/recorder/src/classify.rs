//! Turns an AppSync response into a verdict on the value that was sent.
//!
//! A request can fail for reasons unrelated to the value, such as a broken schema or a
//! resolver error. Recording those as rejections would corrupt the vectors, so only
//! recognized scalar errors count as rejections and anything else is `Unexpected`.

use serde::Serialize;
use serde_json::Value;

use crate::graphql::Path;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Outcome {
    /// `value` is what the resolver saw for input paths, or what the client got for output.
    Accepted { value: Value },
    Rejected { error: String },
    /// The query could not be parsed, so AppSync never reached the scalar check.
    SyntaxError { error: String },
    Unexpected { status: u16, body: String },
}

impl Outcome {
    /// Compares verdicts while ignoring error wording, which may vary between requests.
    pub fn same_verdict(&self, other: &Outcome) -> bool {
        match (self, other) {
            (Outcome::Accepted { value: a }, Outcome::Accepted { value: b }) => a == b,
            (Outcome::Rejected { .. }, Outcome::Rejected { .. }) => true,
            (Outcome::SyntaxError { .. }, Outcome::SyntaxError { .. }) => true,
            (Outcome::Unexpected { .. }, Outcome::Unexpected { .. }) => true,
            _ => false,
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, Outcome::Accepted { .. })
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, Outcome::Rejected { .. })
    }

    pub fn is_unexpected(&self) -> bool {
        matches!(self, Outcome::Unexpected { .. })
    }
}

pub fn classify(path: Path, status: u16, body: &str) -> Outcome {
    let unexpected = || Outcome::Unexpected {
        status,
        body: body.to_string(),
    };

    let Ok(parsed) = serde_json::from_str::<Value>(body) else {
        return unexpected();
    };

    if let Some(first) = parsed["errors"].as_array().and_then(|errors| errors.first()) {
        let message = first["message"].as_str().unwrap_or_default();
        return if is_syntax_error(message) {
            Outcome::SyntaxError {
                error: message.to_string(),
            }
        } else if is_scalar_rejection(message) {
            Outcome::Rejected {
                error: message.to_string(),
            }
        } else {
            unexpected()
        };
    }

    if status != 200 {
        return unexpected();
    }
    let Some(returned) = parsed.get("data").and_then(|data| data.get("r")) else {
        return unexpected();
    };

    match path {
        Path::Output => Outcome::Accepted {
            value: returned.clone(),
        },
        // Input resolvers return the argument as JSON text so its type survives the trip.
        Path::InputVariable | Path::InputLiteral => {
            let decoded = returned
                .as_str()
                .and_then(|text| serde_json::from_str::<Value>(text).ok());
            match decoded {
                Some(value) => Outcome::Accepted { value },
                None => unexpected(),
            }
        }
    }
}

fn is_syntax_error(message: &str) -> bool {
    message.contains("InvalidSyntax") || message.contains("Invalid Syntax")
}

fn is_scalar_rejection(message: &str) -> bool {
    message.starts_with("Validation error of type WrongType")
        || message.starts_with("Variable '")
        || message.starts_with("Can't serialize value")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn errors(message: &str) -> String {
        json!({ "data": null, "errors": [{ "message": message }] }).to_string()
    }

    #[test]
    fn input_decodes_what_the_resolver_saw() {
        let body = json!({ "data": { "r": "\"2021-01-11\"" } }).to_string();
        assert_eq!(
            classify(Path::InputVariable, 200, &body),
            Outcome::Accepted { value: json!("2021-01-11") }
        );
    }

    #[test]
    fn input_keeps_null_and_number_types() {
        let body = json!({ "data": { "r": "null" } }).to_string();
        assert_eq!(
            classify(Path::InputLiteral, 200, &body),
            Outcome::Accepted { value: json!(null) }
        );
        let body = json!({ "data": { "r": "123" } }).to_string();
        assert_eq!(
            classify(Path::InputLiteral, 200, &body),
            Outcome::Accepted { value: json!(123) }
        );
    }

    #[test]
    fn output_keeps_the_returned_value() {
        let body = json!({ "data": { "r": "12:30" } }).to_string();
        assert_eq!(
            classify(Path::Output, 200, &body),
            Outcome::Accepted { value: json!("12:30") }
        );
    }

    #[test]
    fn scalar_errors_are_rejections() {
        for message in [
            "Validation error of type WrongType: argument 'value' is not a valid 'AWSDate'",
            "Variable 'value' has an invalid value",
            "Can't serialize value (/r) : Unable to serialize `x` as a valid Date Object.",
        ] {
            assert!(classify(Path::InputLiteral, 200, &errors(message)).is_rejected(), "{message}");
        }
    }

    #[test]
    fn syntax_errors_are_kept_apart_from_rejections() {
        let outcome = classify(Path::InputLiteral, 200, &errors("Validation error of type InvalidSyntax: x"));
        assert!(matches!(outcome, Outcome::SyntaxError { .. }));
    }

    #[test]
    fn unrelated_errors_never_count_as_rejections() {
        for message in [
            "Validation error of type FieldUndefined: Field 'inputDate' in type 'Query' is undefined",
            "Validation error of type UnknownType: Unknown type AWSDate",
            "A code error occurred in the resolver",
            "Not Authorized to access inputDate on type Query",
        ] {
            assert!(classify(Path::InputLiteral, 200, &errors(message)).is_unexpected(), "{message}");
        }
    }

    #[test]
    fn malformed_responses_are_unexpected() {
        assert!(classify(Path::Output, 200, "<html>").is_unexpected());
        assert!(classify(Path::Output, 200, "{}").is_unexpected());
        assert!(classify(Path::Output, 500, r#"{"data":{"r":"x"}}"#).is_unexpected());
        // Input resolvers always return a string, so anything else means the API is misconfigured.
        assert!(classify(Path::InputVariable, 200, r#"{"data":{"r":null}}"#).is_unexpected());
    }

    #[test]
    fn same_verdict_ignores_error_wording() {
        let a = Outcome::Rejected { error: "x".into() };
        let b = Outcome::Rejected { error: "y".into() };
        assert!(a.same_verdict(&b));
        let c = Outcome::Accepted { value: json!(1) };
        let d = Outcome::Accepted { value: json!(2) };
        assert!(!c.same_verdict(&d));
        assert!(!a.same_verdict(&c));
    }

    #[test]
    fn outcomes_serialize_with_a_result_tag() {
        let value = serde_json::to_value(Outcome::Accepted { value: json!("x") }).unwrap();
        assert_eq!(value, json!({ "result": "accepted", "value": "x" }));
    }
}
