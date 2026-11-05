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
        Control {
            scalar: Scalar::Url,
            good: string("https://example.com/path"),
            bad: string("not a url"),
        },
        Control {
            scalar: Scalar::Email,
            good: string("user@example.com"),
            bad: string("not-an-email"),
        },
        // A number libphonenumber calls valid, in case AppSync uses it: 212 is a real area code,
        // and 555-01xx is reserved for fiction.
        Control {
            scalar: Scalar::Phone,
            good: string("+1 212 555 0100"),
            bad: string("not-a-phone"),
        },
        Control {
            scalar: Scalar::IpAddress,
            good: string("192.168.1.1"),
            bad: string("not-an-ip"),
        },
        // Input resolvers see the parsed document, which preflight allows for a JSON control.
        Control {
            scalar: Scalar::Json,
            good: string("{\"a\":1}"),
            bad: string("{not json"),
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

fn url_cases(b: &mut Builder) {
    let s = Scalar::Url;
    b.strings(s, "documented", [
        "https://example.com", "http://example.com", "mailto:user@example.com",
        "ftp://ftp.example.com/file.txt", "https://www.amazon.com/",
    ]);
    b.strings(s, "scheme", [
        "example.com", "www.example.com", "//example.com", "https://", "http:", "http://",
        "HTTPS://example.com", "Https://example.com", "http:example.com", "http:/example.com",
        "http:///example.com", "http//example.com", "ht tp://example.com", "1http://example.com",
        "ssh://git@example.com", "git+ssh://git@example.com/repo.git", "s3://bucket/key",
        "file:///etc/hosts", "file://example.com/x", "data:text/plain,hello", "tel:+15551234567",
        "urn:isbn:0451450523", "javascript:alert(1)", "custom-scheme://x", "x://", "ws://example.com",
        "wss://example.com/socket", "ldap://example.com/dc=x", "news:comp.lang.rust",
        "gopher://example.com", "sftp://example.com/x", "+http://example.com",
        "http+unix://%2Ftmp%2Fsock/x", "h://x",
    ]);
    b.strings(s, "mailto", [
        "mailto:user@example.com?subject=hi", "mailto:a@b.c,d@e.f", "mailto:", "mailto:not-an-email",
        "mailto://user@example.com", "mailto:user@example.com//x", "MAILTO:user@example.com",
    ]);
    b.strings(s, "path", [
        "https://example.com/", "https://example.com/a/b", "https://example.com/a//b",
        "https://example.com//a", "https://example.com/a/", "https://example.com/a/b/",
        "https://example.com/a///b", "https://example.com/%20", "https://example.com/a b",
        "https://example.com/a%zz", "https://example.com/%", "https://example.com/.",
        "https://example.com/..", "https://example.com/a/../b", "https://example.com/a;b",
        "https://example.com/a|b", "https://example.com/[a]", "https://example.com/a\"b",
        "https://example.com/a<b>", "https://example.com/{a}", "https://example.com/a^b",
        "https://example.com/a`b", "https://example.com/\\a", "https://example.com/a\\b",
        "https://example.com/a,b", "https://example.com/a:b", "https://example.com/a@b",
        "https://example.com/~user", "https://example.com/a!b", "https://example.com/a*b",
        "https://example.com/a'b", "https://example.com/a(b)", "https://example.com/a+b",
        "https://example.com/a=b", "https://example.com/a$b", "https://example.com/a&b",
    ]);
    b.strings(s, "query-fragment", [
        "https://example.com?x", "https://example.com/?", "https://example.com/a?b=c",
        "https://example.com/a?b=c&d=e", "https://example.com/a?b=c//d", "https://example.com/a?b=//",
        "https://example.com/a?q=http://other.com", "https://example.com/a?q=https%3A%2F%2Fx",
        "https://example.com/a?b=c d", "https://example.com/a??b", "https://example.com#x",
        "https://example.com/#", "https://example.com/a#frag", "https://example.com/a#f//g",
        "https://example.com/a#f#g", "https://example.com/a?b#c", "https://example.com/a#c?b",
        "https://example.com/a#", "https://example.com/a? ",
    ]);
    b.strings(s, "host", [
        "https://localhost", "https://localhost:3000", "https://example", "https://example.",
        "https://.example.com", "https://-example.com", "https://example-.com", "https://exa_mple.com",
        "https://ex ample.com", "https://example.com.", "https://sub.sub.sub.example.com",
        "https://EXAMPLE.COM", "https://example.c", "https://example.123", "https://example.com-",
        "https://münchen.de", "https://xn--mnchen-3ya.de", "https://例子.测试", "https://exämple.com/ä",
        "https://a.b", "https://a", "https://1", "https://example..com",
    ]);
    b.strings(s, "port", [
        "https://example.com:443", "https://example.com:80", "https://example.com:0",
        "https://example.com:65535", "https://example.com:65536", "https://example.com:99999",
        "https://example.com:-1", "https://example.com:abc", "https://example.com:",
        "https://example.com:443/", "https://example.com:8080/p?q#f", "https://example.com:08080",
    ]);
    b.strings(s, "ip-host", [
        "https://192.168.1.1", "https://192.168.1.1:8080/x", "https://256.256.256.256",
        "https://0x7f.1", "https://1.2.3", "https://127.1", "https://[2001:db8::1]",
        "https://[2001:db8::1]:8080", "https://2001:db8::1", "https://[::1]", "https://[]",
        "https://[2001:db8::1", "https://[example.com]", "https://0.0.0.0", "https://[::ffff:1.2.3.4]",
    ]);
    b.strings(s, "userinfo", [
        "https://user@example.com", "https://user:pass@example.com", "https://@example.com",
        "https://user:@example.com", "https://:pass@example.com", "https://user:pass@example.com:8080/p?q#f",
        "https://user@:8080", "https://us er@example.com", "https://user:p@ss@example.com",
        "https://user%40x@example.com",
    ]);
    b.strings(s, "whitespace", [
        " https://example.com", "https://example.com ", "https://example.com\n", "https://example.com\t/x",
        "https://example.com/a\u{0}b", "https://example.com/a\nb", "https://example .com",
        "\u{FEFF}https://example.com",
    ]);
    b.strings(s, "unicode", [
        "https://example.com/日本", "https://example.com/?q=日本", "https://example.com/\u{FEFF}",
        "\u{FF48}ttps://example.com", "https://example.com/\u{1F600}", "https://example.com/é",
        "https://example.com/%C3%A9", "https://example.com/\u{2044}a", "https:\u{2215}\u{2215}example.com",
    ]);
    b.strings(s, "length", [
        format!("https://example.com/{}", "a".repeat(2000)),
        format!("https://example.com/{}", "a".repeat(8000)),
        format!("https://example.com/{}", "a".repeat(65000)),
        format!("https://{}.example.com", "a".repeat(63)),
        format!("https://{}.example.com", "a".repeat(64)),
        format!("https://{}.com", ["abcdefghij"; 26].join(".")),
        format!("https://example.com/?{}", "a=b&".repeat(500)),
    ]);
    b.strings(s, "other-scalar", ["user@example.com", "192.168.1.1", "2021-01-11", "{\"a\":1}"]);
    b.junk(s);
}

fn email_cases(b: &mut Builder) {
    let s = Scalar::Email;
    b.strings(s, "documented", ["example@example.com", "john.doe@example.com"]);
    b.strings(s, "local-part", [
        "a@example.com", "first.last@example.com", "first+tag@example.com", "first_last@example.com",
        "first-last@example.com", "first..last@example.com", ".first@example.com", "first.@example.com",
        "!#$%&'*+/=?^_`{|}~@example.com", "first last@example.com", "@example.com",
        "\"quoted\"@example.com", "\"quoted spaced\"@example.com", "\"a@b\"@example.com",
        "\"first..last\"@example.com", "\"\"@example.com", "\"a\\\"b\"@example.com", "a\\@b@example.com",
        "UPPER@EXAMPLE.COM", "MiXeD@Example.Com", "user%example.com@other.com", "user(comment)@example.com",
        "(comment)user@example.com", "user@(comment)example.com", "1234567890@example.com",
        "user,name@example.com", "user:name@example.com", "user;name@example.com", "user<name@example.com",
        "user>name@example.com", "user[name@example.com", "user]name@example.com", "user\"name@example.com",
        "user\\name@example.com", "user\u{0}name@example.com",
        &format!("{}@example.com", "a".repeat(64)),
        &format!("{}@example.com", "a".repeat(65)),
        &format!("{}@example.com", "a".repeat(255)),
    ]);
    b.strings(s, "domain", [
        "user@localhost", "user@example", "user@example.co.uk", "user@sub.sub.sub.example.com",
        "user@example.com.", "user@.example.com", "user@-example.com", "user@example-.com",
        "user@exa_mple.com", "user@example.c", "user@example.123", "user@123.com", "user@123",
        "user@example.museum", "user@example.c0m", "user@ex ample.com", "user@example..com",
        "user@example.com:25", "user@a.b", "user@a", "user@example.verylongtldthatdoesnotexist",
        "user@EXAMPLE.COM", "user@example.com-", "user@e-x-a-m-p-l-e.com",
        &format!("user@{}.com", "a".repeat(63)),
        &format!("user@{}.com", "a".repeat(64)),
        &format!("user@{}.com", ["abcdefghij"; 24].join(".")),
        &format!("user@{}.com", ["abcdefghij"; 26].join(".")),
    ]);
    b.strings(s, "address-literal", [
        "user@[192.168.1.1]", "user@192.168.1.1", "user@[IPv6:2001:db8::1]", "user@[2001:db8::1]",
        "user@2001:db8::1", "user@[256.1.1.1]", "user@[example.com]", "user@[]",
    ]);
    b.strings(s, "unicode", [
        "üser@example.com", "user@münchen.de", "user@xn--mnchen-3ya.de", "用户@例子.广告",
        "user@例子.广告", "user@exämple.com", "\u{FF41}@example.com", "user@example.com\u{FEFF}",
        "\u{FEFF}user@example.com", "user\u{200B}@example.com", "😀@example.com", "user@😀.com",
        "user@example.\u{0441}om",
    ]);
    b.strings(s, "structure", [
        "user", "user@", "user@@example.com", "user@example.com@", "user@example.com@other.com",
        "user@example.com,other@example.com", "user@example.com;", "user@example.com;other@example.com",
        "John Doe <user@example.com>", "<user@example.com>", "\"John Doe\" <user@example.com>",
        "mailto:user@example.com", "user at example dot com", "user@example.com?subject=hi",
        "user@example.com/path", "user@example.com#x",
    ]);
    b.strings(s, "whitespace", [
        " user@example.com", "user@example.com ", "user@example.com\n", "\tuser@example.com",
        "user @example.com", "user@ example.com", "user@example.com\r\n",
    ]);
    b.strings(s, "length", [
        &format!("{}@{}.com", "a".repeat(64), ["abcdefghij"; 17].join(".")),
        &format!("{}@{}.com", "a".repeat(64), ["abcdefghij"; 18].join(".")),
        &format!("{}@example.com", "a".repeat(1000)),
        &format!("user@{}.com", "a".repeat(10000)),
    ]);
    b.strings(s, "other-scalar", ["https://example.com", "192.168.1.1", "+1 212 555 0100", "2021-01-11"]);
    b.junk(s);
}

fn phone_cases(b: &mut Builder) {
    let s = Scalar::Phone;
    b.strings(s, "documented", ["+1 212 555 0100", "212-555-0100", "212 555 0100"]);
    b.strings(s, "nanp-form", [
        "2125550100", "212.555.0100", "(212) 555-0100", "(212)555-0100", "(212) 555 0100",
        "1-212-555-0100", "1 212 555 0100", "12125550100", "+1-212-555-0100", "+12125550100",
        "+1 (212) 555-0100", "+1.212.555.0100", "001-212-555-0100", "011-1-212-555-0100",
        "1 (212) 555-0100", "212-555-0100 x123", "212-555-0100 ext. 123", "212-555-0100 ext 123",
        "212-555-0100;ext=123", "212-555-0100,123", "212-555-0100#123", "+1 212 555 0100 x 123",
        "555-0100", "5550100", "555-010", "212-555-010", "212-555-01000", "212-555-0100-",
        "-212-555-0100", "212--555--0100", "212 - 555 - 0100", "212_555_0100", "212/555/0100",
        "(212 555-0100", "212) 555-0100", "((212)) 555-0100", "+1 212", "+1 212 555", "212",
        "212 555", "+1",
    ]);
    b.strings(s, "nanp-rule", [
        // Area codes and exchanges can't start with 0 or 1, and N11 codes are services.
        "012-555-0100", "112-555-0100", "212-055-0100", "212-155-0100", "211-555-0100",
        "911-555-0100", "212-911-0100", "555-555-5555", "555-123-4567", "000-000-0000",
        "999-999-9999", "800-555-0100", "900-555-0100", "888-555-0100", "500-555-0100",
        "710-555-0100", "911", "411", "988", "211-555-0100", "212-555-0000", "212-555-9999",
        "212-000-0000", "212-555-1212", "416-555-0100", "876-555-0100", "684-555-0100",
    ]);
    b.strings(s, "international", [
        "+44 20 7946 0958", "+442079460958", "+44 (0)20 7946 0958", "+44-20-7946-0958",
        "+44.20.7946.0958", "+(44) 20 7946 0958", "+91 98765 43210", "+86 10 1234 5678",
        "+49 30 123456", "+33 1 23 45 67 89", "+61 2 1234 5678", "+81 3-1234-5678",
        "+7 495 123-45-67", "+55 11 91234-5678", "+1 416 555 0100", "+1 876 555 0100",
        "+1 684 555 0100", "+999 123456", "+0 123456", "+00 44 20 7946 0958", "0044 20 7946 0958",
        "011 44 20 7946 0958", "00 44 20 7946 0958", "+44", "+", "+-1", "++44 20 7946 0958",
        "+44 20 7946 0958 ext 1", "+123456789012345", "+1234567890123456", "+12345678901234567",
        "+1 2", "+12", "+123", "+1234", "+12345", "+123456", "+1234567", "+352 27 12 34",
        "+47 21 23 45 67", "+972 2 123 4567", "+800 1234 5678", "+1 555 123 4567", "+1 212 555 0100 ",
        " +1 212 555 0100", "+ 1 212 555 0100", "+1  212  555  0100",
    ]);
    b.strings(s, "text", [
        "abc", "555-CALL-NOW", "1-800-FLOWERS", "phone", "212-555-0100 (mobile)", "tel:+12125550100",
        "tel:2125550100", "call 212-555-0100", "212-555-0100 or 212-555-0101", "0", "1", "12", "123",
        "1234", "12345", "123456", "1234567", "12345678", "123456789", "1234567890", "12345678901",
        "123456789012", "1234567890123456789012345678901234567890", "+1 212 555 0100\n",
        "212-555-0100\u{0}", "212\t555\t0100", "212\u{00A0}555\u{00A0}0100", "212\u{2013}555\u{2013}0100",
        "212\u{2212}555\u{2212}0100", "\u{0662}\u{0661}\u{0662}-\u{0665}\u{0665}\u{0665}-\u{0660}\u{0661}\u{0660}\u{0660}",
        "\u{FF12}\u{FF11}\u{FF12}-\u{FF15}\u{FF15}\u{FF15}-\u{FF10}\u{FF11}\u{FF10}\u{FF10}",
        "\u{FF0B}1 212 555 0100", "☎ 212-555-0100",
    ]);
    b.strings(s, "length", [
        &"2".repeat(50), &"2".repeat(1000), &format!("+1 {}", "2".repeat(100)),
    ]);
    b.strings(s, "other-scalar", ["user@example.com", "192.168.1.1", "2021-01-11", "1610368224"]);
    b.junk(s);
}

fn ip_address_cases(b: &mut Builder) {
    let s = Scalar::IpAddress;
    b.strings(s, "documented", ["123.12.34.56", "1a2b:3c4b::1234:4567", "123.45.67.89/16"]);
    b.strings(s, "ipv4", [
        "0.0.0.0", "255.255.255.255", "127.0.0.1", "10.0.0.1", "192.168.1.1", "169.254.1.1",
        "224.0.0.1", "256.0.0.1", "1.2.3.256", "999.999.999.999", "1.2.3", "1.2", "1", "1.2.3.4.5",
        "01.2.3.4", "001.002.003.004", "1.2.3.04", "1.2.3.-4", "+1.2.3.4", "1.2.3.4.", ".1.2.3.4",
        "1..2.3.4", "1 .2.3.4", "1. 2.3.4", "1.2.3.4:80", "2130706433", "0x7f.0.0.1", "0x7f000001",
        "127.1", "127.0.1", "1.2.3.4a", "a.b.c.d", "1,2,3,4", "1-2-3-4", "1_2_3_4", "1.2.3.4 ",
        " 1.2.3.4", "1.2.3.4\n", "1.2.3.4\u{0}", "１.２.３.４", "\u{0661}.2.3.4", "1.2.3.4\u{FEFF}",
    ]);
    b.strings(s, "ipv4-cidr", [
        "1.2.3.4/0", "1.2.3.4/8", "1.2.3.4/24", "1.2.3.4/32", "1.2.3.4/33", "1.2.3.4/-1",
        "1.2.3.4/", "1.2.3.4/255.255.255.0", "1.2.3.4/8a", "1.2.3.4 /8", "1.2.3.4/ 8", "1.2.3.4/08",
        "1.2.3.4/16/24", "1.2.3.4//16", "1.2.3.4/16 ", "1.2.3.4/x", "1.2.3.4/999", "1.2.3/16",
        "0.0.0.0/0", "255.255.255.255/32", "/16", "1.2.3.4/+8", "1.2.3.4/1.5", "1.2.3.4/0x8",
        "1.2.3.4\\8",
    ]);
    b.strings(s, "ipv6", [
        "::", "::1", "1::", "2001:db8::1", "2001:0db8:0000:0000:0000:0000:0000:0001",
        "2001:db8:0:0:0:0:0:1", "2001:DB8::1", "2001:Db8::1", "2001:0db8:85a3:0000:0000:8a2e:0370:7334",
        "2001:db8:85a3::8a2e:370:7334", "fe80::1", "ff02::1", "::ffff:192.168.1.1", "::FFFF:192.168.1.1",
        "::ffff:c0a8:101", "::ffff:0:0", "::192.168.1.1", "::ffff:256.1.1.1", "64:ff9b::192.0.2.33",
        "2001:db8::192.168.1.1", "2001:db8::1%eth0", "2001:db8::1%25eth0", "fe80::1%lo0", "fe80::1%1",
        "[2001:db8::1]", "[::1]", "[2001:db8::1]:80", "2001:db8::1:", ":2001:db8::1", "2001:db8:::1",
        "2001:db8::1::2", "2001:db8::12345", "2001:db8::g", "2001:db8:1:2:3:4:5:6:7",
        "2001:db8:1:2:3:4:5", "2001:db8:1:2:3:4:5:6", "1:2:3:4:5:6:7::", "::2:3:4:5:6:7:8",
        "1::2:3:4:5:6:7:8", "2001:db8::1:2:3:4:5:6", "2001:db8::0001", "2001:db8::00001",
        "2001:db8::1 ", " 2001:db8::1", "2001:db8::1\n", "2001-db8--1", "2001.db8..1",
        "2001:db8:: 1", "：：1", "2001:db8::\u{0661}", "::1.2.3", "1:2:3:4:5:6:1.2.3.4",
        "1:2:3:4:5:6:7:1.2.3.4", "::ffff:1.2.3.4.5",
    ]);
    b.strings(s, "ipv6-cidr", [
        "2001:db8::1/64", "2001:db8::1/128", "2001:db8::1/129", "2001:db8::1/0", "2001:db8::/32",
        "::/0", "::1/0", "::1/128", "2001:db8::1/", "2001:db8::1/64a", "2001:db8::1/ 64",
        "2001:db8::1/-1", "2001:db8::1//64", "2001:db8::1/64/0", "2001:db8::1/ffff::",
        "2001:db8::1%eth0/64", "[2001:db8::1]/64", "2001:db8::1/064", "2001:db8::1/999",
    ]);
    b.strings(s, "other-scalar", [
        "localhost", "example.com", "https://1.2.3.4", "user@1.2.3.4", "1.2.3.4/16 extra",
        "2021-01-11", "1610368224",
    ]);
    b.strings(s, "length", [
        &format!("1.2.3.4/{}", "8".repeat(100)),
        &format!("{}.2.3.4", "1".repeat(1000)),
        &format!("2001:db8::{}", "1".repeat(1000)),
    ]);
    b.junk(s);
}

fn json_cases(b: &mut Builder) {
    let s = Scalar::Json;
    // The documented form: JSON text in a string.
    b.strings(s, "documented", ["{\"a\":1}", "{\"key\": \"value\", \"list\": [1, 2]}"]);
    b.strings(s, "json-text", [
        "{}", "[]", "[1,2,3]", "\"string\"", "\"\"", "123", "-1.5", "1e3", "1E3", "true", "false",
        "null", "{\"a\":{\"b\":[1,{\"c\":null}]}}", "{\"a\":1,\"b\":2}", "[[[]]]", "[{}]", "{\"a\":[]}",
        "{\"\":1}", "{\"a\":\"\"}", "{\"a\":1,\"a\":2}", "{\"a\":\"\\u0000\"}", "\"\\u00e9\"", "\"é\"",
"\"\\ud83d\\ude00\"", "\"😀\"", "{\"😀\":1}", "\"\\n\"", "\"\\/\"", "\"\\x41\"",
        "\"\\u00\"", "9223372036854775808", "18446744073709551616", "1e400", "-0", "0.1e-7",
        "1.0", "1.50", "100000000000000000000000000000", "0.00000000000000000000000000001",
        "123456789012345678901234567890.123456789", "\"2021-01-11\"", "\"1\"",
    ]);
    b.strings(s, "json-text-whitespace", [
        " {}", "{} ", " {} ", "{}\n", "\n{}", "{ }", "{\"a\" : 1}", "{\"a\":1 }", "[ 1 , 2 ]", "\t{}",
        "{}\r\n", "{}\u{0}", "\u{FEFF}{}", "{}\u{00A0}", "{\u{00A0}}",
    ]);
    b.strings(s, "json-text-invalid", [
        "", " ", "{", "}", "[", "]", "{\"a\":1,}", "[1,2,]", "[,]", "[1 2]", "{\"a\" 1}", "{'a':1}",
        "{a:1}", "{\"a\":1}{\"b\":2}", "{\"a\":1} x", "{}{}", "[][]", "unquoted", "undefined", "NaN",
        "Infinity", "-Infinity", "{\"a\":NaN}", "[NaN]", "[Infinity]", "01", "1.", ".5", "+1", "0x10",
        "1_000", "1,000", "'string'", "\"unterminated", "{\"a\":\"b\\\"}", "{1:2}", "{null:1}",
        "{true:1}", "{\"a\":}", "{:1}", "{\"a\"}", "[1,,2]", "[\"a\" \"b\"]", "{\"a\":1}//c",
        "/*c*/{}", "{\"a\":1} // comment", "{\"a\":1 /* c */}", "# {}", "{\"a\":01}", "{\"a\":1.}",
        "{\"a\":.5}", "{\"a\":+1}", "{\"a\":'b'}", "{\"a\":\"b\n\"}", "{\"a\":\"b\t\"}", "{\"a\":\"\\u\"}",
        "{\"a\":\"\\uzzzz\"}", "{\"a\":\"\\q\"}", "True", "FALSE", "Null", "nil", "None", "{}}", "[]]",
        "{[]}", "[{]", "{\"a\":[}", "<xml/>", "a=1", "{\"a\":1\n}", "\"a\\\n\"",
    ]);
    b.strings(s, "json-text-nesting", [
        &format!("{}{}", "[".repeat(100), "]".repeat(100)),
        &format!("{}{}", "[".repeat(1000), "]".repeat(1000)),
        &format!("{}1{}", "{\"a\":".repeat(100), "}".repeat(100)),
        &format!("{}1{}", "{\"a\":".repeat(1000), "}".repeat(1000)),
        &format!("{}{}", "[".repeat(100), "]".repeat(99)),
    ]);
    b.strings(s, "json-text-length", [
        &format!("\"{}\"", "a".repeat(10000)),
        &format!("\"{}\"", "a".repeat(100000)),
        &format!("[{}1]", "1,".repeat(10000)),
        &format!("{{{}\"z\":1}}", (0..1000).map(|i| format!("\"k{i}\":{i},")).collect::<String>()),
    ]);
    // Raw JSON values sent as the variable, rather than text in a string.
    b.json(s, "json-value", &[
        "{}", "[]", "{\"a\":1}", "[1,2]", "{\"a\":{\"b\":[1,{\"c\":null}]}}", "1", "1.5", "-1", "1e3",
        "9223372036854775808", "true", "false", "null", "[\"a\"]", "{\"a\":\"{\\\"b\\\":1}\"}",
    ]);
    b.strings(s, "other-scalar", ["2021-01-11", "https://example.com", "user@example.com"]);
    b.strings(s, "junk", JUNK.iter().copied());
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
    url_cases(&mut builder);
    email_cases(&mut builder);
    phone_cases(&mut builder);
    ip_address_cases(&mut builder);
    json_cases(&mut builder);
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
        assert!(has(&cases, Scalar::Url, &json!("https://example.com/a//b")));
        assert!(has(&cases, Scalar::Url, &json!("example.com")));
        assert!(has(&cases, Scalar::Email, &json!("first+tag@example.com")));
        assert!(has(&cases, Scalar::Phone, &json!("212-555-0100")));
        assert!(has(&cases, Scalar::Phone, &json!("+44 20 7946 0958")));
        assert!(has(&cases, Scalar::IpAddress, &json!("123.45.67.89/16")));
        assert!(has(&cases, Scalar::IpAddress, &json!("[2001:db8::1]")));
        assert!(has(&cases, Scalar::Json, &json!("unquoted")));
    }

    #[test]
    fn json_text_and_json_values_are_separate_cases() {
        let cases = all();
        // The string "{}" and the empty object are different inputs, and both are sent.
        assert!(has(&cases, Scalar::Json, &json!("{}")));
        assert!(has(&cases, Scalar::Json, &json!({})));
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
