//! The values sent to AppSync. To add a case, add it here and record again.
//!
//! Nothing here says what AppSync should do with a value. The recorded vectors are the answer.

use std::collections::HashSet;

use serde_json::Value;

use crate::graphql::Scalar;

#[derive(Clone, Debug)]
pub struct Case {
    pub scalar: Scalar,
    pub group: &'static str,
    pub value: Value,
}

/// A value each path must accept and one each must reject, checked before recording.
pub struct Control {
    pub scalar: Scalar,
    pub good: Value,
    pub bad: Value,
}

pub fn controls() -> Vec<Control> {
    let string = |s: &str| Value::String(s.to_string());
    vec![
        Control {
            scalar: Scalar::Date,
            good: string("2021-01-11"),
            bad: string("not-a-date"),
        },
        Control {
            scalar: Scalar::Time,
            good: string("12:30:24"),
            bad: string("not-a-time"),
        },
        Control {
            scalar: Scalar::DateTime,
            good: string("2021-01-11T12:30:24Z"),
            bad: string("not-a-datetime"),
        },
        Control {
            scalar: Scalar::Timestamp,
            good: serde_json::from_str("1610368224").unwrap(),
            bad: string("not-a-timestamp"),
        },
    ]
}

const OFFSETS: &[&str] = &[
    // Common forms
    "Z", "+00:00", "-00:00", "+05:30", "-07:00",
    // Offsets with seconds, which the docs call out as allowed
    "+00:00:00", "-00:00:00", "+05:30:15", "-07:00:59",
    // Range limits
    "+14:00", "-12:00", "+18:00", "-18:00", "+18:00:00", "-18:00:00", "+18:01", "-18:01",
    "+18:00:01", "-18:00:01", "+19:00", "-19:00", "+23:59", "-23:59", "+24:00", "+99:00",
    "+99:99", "+05:59", "+05:60", "+05:99", "+05:30:59", "+05:30:60", "+05:30:99",
    // Hours alone, which AWSDateTime accepts
    "+00", "-00", "+18", "-18", "+19", "-19", "+23", "+24", "+99", "+5",
    // Field limits at their largest
    "+23:59:59", "-23:59:59", "+24:00:00", "+00:60", "+00:00:60",
    // Other spellings
    "z", "+05", "-05", "+0530", "-0530", "+053015", "+5:30", "+05:3", "+05:", "+05:30:",
    "+05:30:1", "+05:30:15.5", "+05:30:15Z", "Z+00:00", "ZZ", " Z", " +05:30", "+ 05:30",
    "-07:00Z", "UTC", "GMT", "UTC+1", "PST",
    // Forms java.time.format.DateTimeFormatter.ISO_DATE_TIME understands
    "[UTC]", "Z[UTC]", "+01:00[Europe/Paris]",
    // Lookalike characters
    "\u{2212}05:30", "\u{b1}05:30", "+\u{0660}\u{0665}:\u{0663}\u{0660}",
];

const JUNK: &[&str] = &[
    "", " ", "null", "undefined", "NaN", "Infinity", "now", "today", "abc", "0", "-1",
    "\u{1F600}",
];

const WRONG_TYPES: &[&str] = &[
    "null", "true", "false", "0", "1", "20210111", "1610368224", "1.5", "1e3", "[]",
    "[\"2021-01-11\"]", "{}", "{\"a\":1}",
];

struct Builder {
    cases: Vec<Case>,
    seen: HashSet<(&'static str, String)>,
}

impl Builder {
    fn value(&mut self, scalar: Scalar, group: &'static str, value: Value) {
        // The first group to use a value keeps it, so groups read from most to least specific.
        if self.seen.insert((scalar.type_name(), value.to_string())) {
            self.cases.push(Case { scalar, group, value });
        }
    }

    fn strings<S: Into<String>>(
        &mut self,
        scalar: Scalar,
        group: &'static str,
        values: impl IntoIterator<Item = S>,
    ) {
        for value in values {
            self.value(scalar, group, Value::String(value.into()));
        }
    }

    fn json(&mut self, scalar: Scalar, group: &'static str, raw: &[&str]) {
        for text in raw {
            let value = serde_json::from_str(text).expect("test case is valid JSON");
            self.value(scalar, group, value);
        }
    }

    fn junk(&mut self, scalar: Scalar) {
        self.strings(scalar, "junk", JUNK.iter().copied());
        self.json(scalar, "wrong-type", WRONG_TYPES);
    }
}

/// Date text in every form worth trying, shared by AWSDate and AWSDateTime.
fn date_bodies() -> Vec<(&'static str, String)> {
    let mut bodies: Vec<(&'static str, String)> = Vec::new();
    let mut add = |group: &'static str, values: &[&str]| {
        bodies.extend(values.iter().map(|v| (group, v.to_string())));
    };

    add("year-form", &[
        "2021-01-11", "1970-01-11", "0001-01-11", "0000-01-11", "9999-01-11", "-0001-01-11",
        "-2017-01-11", "-9999-01-11", "-0000-01-11", "+2021-01-11", "+02021-01-11",
        "+12021-01-11", "12021-01-11", "-12021-01-11", "021-01-11", "21-01-11", "202-01-11",
        "20 21-01-11",
    ]);
    add("year-width", &[
        "+0000-01-11", "+00000-01-11", "-00000-01-11", "+00001-01-11", "-02021-01-11",
        "+100000-01-11", "-100000-01-11", "123456789-01-11", "+999999999-01-11",
        "-999999999-01-11", "+0999999999-01-11", "-0999999999-01-11", "+1000000000-01-11",
        "-1000000000-01-11", "+9999999999-01-11", "+00000000000-01-11", "+10000000000-01-11",
    ]);
    add("long-year-leap", &[
        "+10000-02-29", "-10000-02-29", "+12024-02-29", "+10100-02-29", "-12023-02-29",
    ]);
    add("month-range", &[
        "2021-00-11", "2021-12-11", "2021-13-11", "2021-99-11", "2021-1-11", "2021-001-11",
        "2021-Jan-11", "2021--1-11",
    ]);
    add("day-range", &["2021-01-00", "2021-01-32", "2021-01-99", "2021-01-1", "2021-01-011"]);
    add("month-length", &[
        "2021-01-31", "2021-02-28", "2021-02-29", "2021-02-30", "2021-03-31", "2021-03-32",
        "2021-04-30", "2021-04-31", "2021-05-31", "2021-05-32", "2021-06-30", "2021-06-31",
        "2021-07-31", "2021-07-32", "2021-08-31", "2021-08-32", "2021-09-30", "2021-09-31",
        "2021-10-31", "2021-10-32", "2021-11-30", "2021-11-31", "2021-12-31", "2021-12-32",
    ]);
    add("leap-year", &[
        "2020-02-29", "2000-02-29", "1900-02-29", "2100-02-29", "1600-02-29", "1000-02-29",
        "0004-02-29", "0000-02-29", "-0004-02-29", "-0001-02-29", "-0400-02-29",
    ]);
    // Parsers using a Julian/Gregorian hybrid calendar treat these October 1582 dates as missing.
    add("calendar-cutover", &[
        "1582-10-04", "1582-10-05", "1582-10-10", "1582-10-14", "1582-10-15",
    ]);
    add("alternate-format", &[
        "2021/01/11", "20210111", "2021-01", "2021", "202101", "2021-W02-1", "2021W021",
        "2021-011", "01-11-2021", "11/01/2021", "Jan 11 2021", "2021.01.11", "2021-1-1",
    ]);
    add("whitespace", &[
        " 2021-01-11", "2021-01-11 ", "2021-01-11\n", "\t2021-01-11", "2021-01-11\u{0}",
        "2021 -01-11",
    ]);
    add("lookalike-characters", &[
        "\u{0662}\u{0660}\u{0662}\u{0661}-\u{0660}\u{0661}-\u{0661}\u{0661}",
        "\u{FF12}\u{FF10}\u{FF12}\u{FF11}-\u{FF10}\u{FF11}-\u{FF11}\u{FF11}",
        "2021\u{2013}01\u{2013}11",
        "2021\u{2212}01\u{2212}11",
        "\u{FEFF}2021-01-11",
    ]);
    bodies
}

/// Time text in every form worth trying, shared by AWSTime and AWSDateTime.
fn time_bodies() -> Vec<(&'static str, String)> {
    let mut bodies: Vec<(&'static str, String)> = Vec::new();
    let mut add = |group: &'static str, values: &[&str]| {
        bodies.extend(values.iter().map(|v| (group, v.to_string())));
    };

    add("clock-range", &[
        "00:00", "23:59", "24:00", "24:01", "25:00", "12:59", "12:60", "12:99", "00:00:00",
        "23:59:59", "23:59:60", "23:59:61", "24:00:00", "24:00:01", "12:00:60", "12:00:99",
        "24:00:00.000", "24:00:00.001",
    ]);
    add("field-width", &[
        "1:00", "12:0", "012:00", "12:000", "12:00:0", "12:00:000", "1:2:3", "12", "1200",
        "120000", "12:", "12:00:", ":00", "12::00",
    ]);
    add("fraction-digits", &[
        "12:00:00.1", "12:00:00.12", "12:00:00.123", "12:00:00.1234", "12:00:00.12345",
        "12:00:00.123456", "12:00:00.1234567", "12:00:00.12345678", "12:00:00.123456789",
        "12:00:00.1234567890", "12:00:00.123456789012", "12:00:00.12345678901234567890",
    ]);
    add("fraction-form", &[
        "12:00:34.", "12:00:00,5", "12:00:00.x", "12:00:00.1x", "12:00:00x5", "12:00:00. 5",
        "12:00:00.-1", "12:00:00.+1", "12:00:00.\u{0663}", "12:30.5", "12.5", "12:00:00..5",
        "12:30.", "12.",
    ]);
    add("alternate-format", &[
        "12:00 PM", "12:00AM", "T12:00", "T12:00:00", "t12:00", "noon", "12h00", "12-00",
        "12.00.00",
    ]);
    add("whitespace", &[" 12:00:00", "12:00:00 ", "12:00:00\n", "\t12:00:00", "12: 00:00"]);
    add("lookalike-characters", &[
        "\u{0661}\u{0662}:\u{0660}\u{0660}",
        "\u{FF11}\u{FF12}:\u{FF10}\u{FF10}",
        "12\u{FF1A}00",
    ]);
    bodies
}

fn date_cases(b: &mut Builder) {
    let s = Scalar::Date;
    b.strings(s, "documented", [
        "1970-01-01", "1970-01-01Z", "1970-01-01-07:00", "1970-01-01+05:30", "-2017-05-01",
        "-9999-01-01",
    ]);
    for (group, value) in date_bodies() {
        b.strings(s, group, [value]);
    }
    b.strings(s, "offset", OFFSETS.iter().map(|o| format!("2021-01-11{o}")));
    b.strings(s, "other-scalar", ["2021-01-11T00:00:00Z", "2021-01-11T", "12:30:24"]);
    b.junk(s);
}

fn time_cases(b: &mut Builder) {
    let s = Scalar::Time;
    b.strings(s, "documented", [
        "12:00:00.2", "12:00:00.277", "12:00:00.123456789", "12:30", "12:30Z", "12:30:24Z",
        "12:30:24-07:00", "12:30:24.500+05:30", "12:30:24.500+05:30:00",
    ]);
    for (group, value) in time_bodies() {
        b.strings(s, group, [value]);
    }
    b.strings(s, "offset", OFFSETS.iter().map(|o| format!("12:30:24{o}")));
    b.strings(s, "offset-without-seconds", [
        "12:30+05:30", "12:30+05:30:15", "12:30-07:00", "12:30+05", "12:30z",
    ]);
    b.strings(s, "empty-fraction-offset", ["12:00:34.Z", "12:00:34.z", "12:00:34.+05:30"]);
    b.strings(s, "other-scalar", ["2021-01-11T12:30:24Z", "2021-01-11"]);
    b.junk(s);
}

fn datetime_cases(b: &mut Builder) {
    let s = Scalar::DateTime;
    b.strings(s, "documented", [
        "1930-01-01T16:00:00-07:00", "1970-01-01T12:00:00.2Z", "1970-01-01T12:00:00.277Z",
        "1970-01-01T12:00:00.123456789Z", "-2017-01-01T00:00Z", "-9999-01-01T00:00Z",
    ]);
    b.strings(s, "offset-required", [
        "2021-01-11T12:30", "2021-01-11T12:30:24", "2021-01-11T12:30:24.5",
        "2021-01-11T12:30:24.123456789",
    ]);
    b.strings(s, "precision", [
        "2021-01-11T12:30Z", "2021-01-11T12:30:24Z", "2021-01-11T12:30:24.5Z",
        "2021-01-11T12:30+05:30", "2021-01-11T12:30+05:30:15", "2021-01-11T12:30.5Z",
        "2021-01-11T12:30+05", "2021-01-11T12:30:24.5+05", "2021-01-11T12:00:34.+05:30",
        "2021-01-11t12:30:24z", "2021-01-11t12:30+05:30",
    ]);
    for separator in ["t", " ", "_", "-", "TT", "Tt", "", "T ", " T", "\n", "\t", "\u{2003}"] {
        b.strings(s, "separator", [format!("2021-01-11{separator}12:30:24Z")]);
    }
    for (group, value) in date_bodies() {
        b.strings(s, group, [format!("{value}T12:30:24Z")]);
    }
    for (group, value) in time_bodies() {
        b.strings(s, group, [format!("2021-01-11T{value}Z")]);
    }
    b.strings(s, "offset", OFFSETS.iter().map(|o| format!("2021-01-11T12:30:24{o}")));
    b.strings(s, "day-rollover", [
        "2021-12-31T24:00:00Z", "2016-12-31T23:59:60Z", "2021-01-11T23:59:60Z",
        "0000-01-01T00:00:00+18:00", "-9999-01-01T00:00:00+18:00",
        "9999-12-31T23:59:59-18:00", "+10000-01-01T00:00:00Z", "-10000-01-01T00:00:00Z",
        "+999999999-12-31T23:59:59.999999999-18:00", "+999999999-12-31T23:59:59.999999999+18:00",
        "-999999999-01-01T00:00:00+18:00", "-999999999-01-01T00:00:00-18:00",
        "+1000000000-12-31T23:59:59.999999999Z", "-1000000000-01-01T00:00Z",
    ]);
    b.strings(s, "other-scalar", [
        "2021-01-11", "2021-01-11Z", "2021-01-11+05:30", "12:30:24Z", "T12:30:24Z",
    ]);
    b.strings(s, "alternate-format", [
        "20210111T123024Z", "2021-01-11T123024Z", "20210111T12:30:24Z",
        "Mon, 11 Jan 2021 12:30:24 GMT", "Mon Jan 11 2021 12:30:24 GMT+0000", "1610368224",
        "2021-01-11T12:30:24.500000000000Z",
    ]);
    b.strings(s, "whitespace", [
        " 2021-01-11T12:30:24Z", "2021-01-11T12:30:24Z ", "2021-01-11T12:30:24Z\n",
    ]);
    b.junk(s);
}

fn timestamp_cases(b: &mut Builder) {
    let s = Scalar::Timestamp;
    b.json(s, "documented", &["-123123"]);
    b.json(s, "integer-range", &[
        "0", "1", "-1", "1610368224", "2147483647", "2147483648", "-2147483648", "-2147483649",
        "4294967296", "253402300799", "-62135596800", "9007199254740991", "9007199254740992",
        "9223372036854775807", "9223372036854775808", "-9223372036854775808",
        "-9223372036854775809", "18446744073709551616", "18446744073709551617",
        "-18446744073709551616", "31556889864403199",
        "31556889864403200", "-31557014167219200", "-31557014167219201",
        "1000000000000000000000000",
    ]);
    b.json(s, "number-form", &[
        "0.0", "1.0", "-1.0", "-0", "-0.0", "1.5", "1e3", "1E3", "1.0e3", "1e-3",
        "1610368224.0", "1610368224.5", "1e400",
    ]);
    b.strings(s, "numeric-string", [
        "1610368224", "1", "-1", "1.5", "1e3", " 1", "1 ", "0x10", "+1", "01",
    ]);
    b.strings(s, "date-string", ["2021-01-11T12:30:24Z", "2021-01-11"]);
    b.strings(s, "junk", JUNK.iter().copied());
    b.json(s, "wrong-type", &["null", "true", "false", "[]", "[1]", "{}", "{\"a\":1}"]);
}

pub fn all() -> Vec<Case> {
    let mut builder = Builder {
        cases: Vec::new(),
        seen: HashSet::new(),
    };
    date_cases(&mut builder);
    time_cases(&mut builder);
    datetime_cases(&mut builder);
    timestamp_cases(&mut builder);
    builder.cases
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn has(cases: &[Case], scalar: Scalar, value: &Value) -> bool {
        cases.iter().any(|c| c.scalar == scalar && &c.value == value)
    }

    #[test]
    fn values_are_unique_per_scalar() {
        let cases = all();
        let unique: HashSet<_> = cases
            .iter()
            .map(|c| (c.scalar.type_name(), c.value.to_string()))
            .collect();
        assert_eq!(unique.len(), cases.len());
    }

    #[test]
    fn every_scalar_has_cases() {
        let cases = all();
        for scalar in Scalar::ALL {
            assert!(cases.iter().filter(|c| c.scalar == scalar).count() > 30, "{scalar:?}");
        }
    }

    #[test]
    fn covers_the_disputed_values() {
        let cases = all();
        assert!(has(&cases, Scalar::DateTime, &json!("2021-01-11T12:30:24")));
        assert!(has(&cases, Scalar::DateTime, &json!("2021-01-11T12:30:24+0530")));
        assert!(has(&cases, Scalar::DateTime, &json!("-2017-01-01T00:00Z")));
        assert!(has(&cases, Scalar::Time, &json!("12:00:34.")));
        assert!(has(&cases, Scalar::Date, &json!("2021-02-29")));
    }

    #[test]
    fn large_numbers_keep_their_exact_text() {
        let cases = all();
        let big = serde_json::from_str("18446744073709551616").unwrap();
        assert!(has(&cases, Scalar::Timestamp, &big));
        let huge = serde_json::from_str("1e400").unwrap();
        assert!(has(&cases, Scalar::Timestamp, &huge));
    }

    #[test]
    fn controls_differ_for_every_scalar() {
        let controls = controls();
        assert_eq!(controls.len(), Scalar::ALL.len());
        for control in controls {
            assert_ne!(control.good, control.bad);
        }
    }
}
