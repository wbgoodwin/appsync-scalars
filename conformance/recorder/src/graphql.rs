//! Builds the GraphQL requests sent to the echo API.

use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scalar {
    Date,
    Time,
    DateTime,
    Timestamp,
    Url,
    Email,
    Phone,
    IpAddress,
    Json,
}

impl Scalar {
    pub const ALL: [Scalar; 9] = [
        Scalar::Date,
        Scalar::Time,
        Scalar::DateTime,
        Scalar::Timestamp,
        Scalar::Url,
        Scalar::Email,
        Scalar::Phone,
        Scalar::IpAddress,
        Scalar::Json,
    ];

    pub fn type_name(self) -> &'static str {
        match self {
            Scalar::Date => "AWSDate",
            Scalar::Time => "AWSTime",
            Scalar::DateTime => "AWSDateTime",
            Scalar::Timestamp => "AWSTimestamp",
            Scalar::Url => "AWSURL",
            Scalar::Email => "AWSEmail",
            Scalar::Phone => "AWSPhone",
            Scalar::IpAddress => "AWSIPAddress",
            Scalar::Json => "AWSJSON",
        }
    }

    pub fn file_stem(self) -> &'static str {
        match self {
            Scalar::Date => "aws-date",
            Scalar::Time => "aws-time",
            Scalar::DateTime => "aws-datetime",
            Scalar::Timestamp => "aws-timestamp",
            Scalar::Url => "aws-url",
            Scalar::Email => "aws-email",
            Scalar::Phone => "aws-phone",
            Scalar::IpAddress => "aws-ipaddress",
            Scalar::Json => "aws-json",
        }
    }

    pub fn from_type_name(name: &str) -> Option<Scalar> {
        Scalar::ALL.into_iter().find(|s| s.type_name() == name)
    }

    fn field_suffix(self) -> &'static str {
        match self {
            Scalar::Date => "Date",
            Scalar::Time => "Time",
            Scalar::DateTime => "DateTime",
            Scalar::Timestamp => "Timestamp",
            Scalar::Url => "Url",
            Scalar::Email => "Email",
            Scalar::Phone => "Phone",
            Scalar::IpAddress => "IpAddress",
            Scalar::Json => "Json",
        }
    }
}

/// The three ways a value can reach AppSync's scalar checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// Sent as a GraphQL variable.
    InputVariable,
    /// Written inline in the query text.
    InputLiteral,
    /// Returned from a resolver for a field typed as the scalar.
    Output,
}

impl Path {
    pub const ALL: [Path; 3] = [Path::InputVariable, Path::InputLiteral, Path::Output];
}

pub fn request(scalar: Scalar, path: Path, value: &Value) -> String {
    let name = scalar.type_name();
    let suffix = scalar.field_suffix();
    let body = match path {
        Path::InputVariable => json!({
            "query": format!("query($value: {name}) {{ r: input{suffix}(value: $value) }}"),
            "variables": { "value": value },
        }),
        Path::InputLiteral => json!({
            "query": format!("query {{ r: input{suffix}(value: {}) }}", literal(value)),
        }),
        Path::Output => json!({
            "query": format!("query($json: String!) {{ r: output{suffix}(json: $json) }}"),
            "variables": { "json": value.to_string() },
        }),
    };
    body.to_string()
}

pub fn descriptions_request() -> String {
    let fields: Vec<String> = Scalar::ALL
        .iter()
        .map(|s| {
            let name = s.type_name();
            format!("{name}: __type(name: \"{name}\") {{ name description }}")
        })
        .collect();
    json!({ "query": format!("query {{ {} }}", fields.join(" ")) }).to_string()
}

/// Renders a JSON value as a GraphQL literal. JSON string escapes are also valid in GraphQL.
pub fn literal(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(literal).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(fields) => {
            let fields: Vec<String> = fields
                .iter()
                .map(|(key, value)| format!("{key}: {}", literal(value)))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
        scalar => scalar.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(body: &str) -> Value {
        serde_json::from_str(body).unwrap()
    }

    #[test]
    fn literal_escapes_strings() {
        assert_eq!(literal(&json!("a\"b\n")), r#""a\"b\n""#);
        assert_eq!(literal(&json!("٢٠٢١")), "\"٢٠٢١\"");
    }

    #[test]
    fn literal_keeps_number_text() {
        for raw in ["1e+3", "-0.0", "1.0", "9223372036854775808", "18446744073709551616"] {
            let value: Value = serde_json::from_str(raw).unwrap();
            assert_eq!(literal(&value), raw);
        }
    }

    #[test]
    fn exponents_gain_a_sign_when_parsed() {
        let value: Value = serde_json::from_str("1e3").unwrap();
        assert_eq!(literal(&value), "1e+3");
    }

    #[test]
    fn literal_renders_compound_values() {
        assert_eq!(literal(&json!([1, "a"])), r#"[1, "a"]"#);
        assert_eq!(literal(&json!({"a": 1})), "{a: 1}");
        assert_eq!(literal(&json!(null)), "null");
        assert_eq!(literal(&json!(true)), "true");
    }

    #[test]
    fn variable_request_sends_the_value_untouched() {
        let body = parsed(&request(Scalar::Date, Path::InputVariable, &json!("2021-01-11")));
        assert_eq!(body["query"], "query($value: AWSDate) { r: inputDate(value: $value) }");
        assert_eq!(body["variables"]["value"], "2021-01-11");
    }

    #[test]
    fn literal_request_inlines_the_value() {
        let body = parsed(&request(Scalar::DateTime, Path::InputLiteral, &json!("x")));
        assert_eq!(body["query"], r#"query { r: inputDateTime(value: "x") }"#);
        assert!(body.get("variables").is_none());
    }

    #[test]
    fn output_request_sends_json_text() {
        let body = parsed(&request(Scalar::Time, Path::Output, &json!("12:30")));
        assert_eq!(body["query"], "query($json: String!) { r: outputTime(json: $json) }");
        assert_eq!(body["variables"]["json"], r#""12:30""#);

        let big: Value = serde_json::from_str("9223372036854775808").unwrap();
        let body = parsed(&request(Scalar::Timestamp, Path::Output, &big));
        assert_eq!(body["variables"]["json"], "9223372036854775808");
    }

    #[test]
    fn descriptions_request_names_every_scalar() {
        let body = parsed(&descriptions_request());
        for scalar in Scalar::ALL {
            assert!(body["query"].as_str().unwrap().contains(scalar.type_name()));
        }
    }
}
