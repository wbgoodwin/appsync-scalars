use appsync_scalars::{Reason, Scalar, validate};

fn reason(scalar: Scalar, value: &str) -> Option<Reason> {
    validate(scalar, value).err().map(|error| error.reason())
}

#[test]
fn date_reports_what_is_wrong() {
    assert_eq!(reason(Scalar::Date, "2021-13-01"), Some(Reason::Month));
    assert_eq!(reason(Scalar::Date, "2021-00-01"), Some(Reason::Month));
    assert_eq!(reason(Scalar::Date, "2021-04-31"), Some(Reason::Day));
    assert_eq!(reason(Scalar::Date, "2021-01-00"), Some(Reason::Day));
    assert_eq!(reason(Scalar::Date, "2021-01-01+0530"), Some(Reason::Offset));
    assert_eq!(reason(Scalar::Date, "2021/01/01"), Some(Reason::Format));
    assert_eq!(reason(Scalar::Date, "2021-01-011"), Some(Reason::Format));
    assert_eq!(reason(Scalar::Date, ""), Some(Reason::Format));
}

#[test]
fn leap_years_follow_the_gregorian_rule() {
    for year in ["2020", "2000", "1600", "0000", "-0004", "-0400"] {
        assert_eq!(reason(Scalar::Date, &format!("{year}-02-29")), None, "{year}");
    }
    for year in ["2021", "1900", "2100", "1000", "-0001", "-0100"] {
        assert_eq!(reason(Scalar::Date, &format!("{year}-02-29")), Some(Reason::Day), "{year}");
    }
}

#[test]
fn years_can_be_longer_with_a_sign() {
    for year in ["0000", "-0001", "+02021", "+00000", "-12021", "+999999999", "-0999999999"] {
        assert_eq!(reason(Scalar::Date, &format!("{year}-01-11")), None, "{year}");
    }
    for year in ["-0000", "-00000", "+2021", "12021", "021", "+", "+99999999999"] {
        assert_eq!(reason(Scalar::Date, &format!("{year}-01-11")), Some(Reason::Format), "{year}");
    }
    for year in ["+1000000000", "-1000000000", "+9999999999"] {
        assert_eq!(reason(Scalar::Date, &format!("{year}-01-11")), Some(Reason::Year), "{year}");
    }
}

#[test]
fn long_years_follow_the_leap_rule_too() {
    assert_eq!(reason(Scalar::Date, "+10000-02-29"), None);
    assert_eq!(reason(Scalar::Date, "-12024-02-29"), None);
    assert_eq!(reason(Scalar::Date, "+10100-02-29"), Some(Reason::Day));
}

#[test]
fn time_reports_what_is_wrong() {
    assert_eq!(reason(Scalar::Time, "24:00"), Some(Reason::Hour));
    assert_eq!(reason(Scalar::Time, "12:60"), Some(Reason::Minute));
    assert_eq!(reason(Scalar::Time, "12:00:60"), Some(Reason::Second));
    assert_eq!(reason(Scalar::Time, "12:00:00."), None);
    assert_eq!(reason(Scalar::Time, "12:30."), Some(Reason::Format));
    assert_eq!(reason(Scalar::Time, "12:00:00.1234567890"), Some(Reason::Fraction));
    assert_eq!(reason(Scalar::Time, "12:30.5"), Some(Reason::Format));
    assert_eq!(reason(Scalar::Time, "12:00:00x5"), Some(Reason::Format));
}

#[test]
fn offsets_need_a_colon_and_in_range_fields() {
    for offset in ["Z", "z", "+00:00", "-07:00", "+05:30:15", "+18:01", "+23:59:59"] {
        assert_eq!(reason(Scalar::Time, &format!("12:30{offset}")), None, "{offset}");
    }
    for offset in ["+05", "+0530", "+5:30", "+05:3", "+24:00", "+05:60", "+05:30:60", "+05:30:"] {
        assert_eq!(reason(Scalar::Time, &format!("12:30{offset}")), Some(Reason::Offset), "{offset}");
    }
    for offset in ["ZZ", " Z", "UTC", "+05:30Z", "[UTC]"] {
        assert_eq!(reason(Scalar::Time, &format!("12:30{offset}")), Some(Reason::Format), "{offset}");
    }
}

#[test]
fn datetime_requires_an_offset() {
    assert_eq!(reason(Scalar::DateTime, "2021-01-11T12:30:24"), Some(Reason::OffsetRequired));
    assert_eq!(reason(Scalar::DateTime, "2021-01-11T12:30:24Z"), None);
}

#[test]
fn datetime_checks_each_part() {
    assert_eq!(reason(Scalar::DateTime, "2021-02-30T12:30:24Z"), Some(Reason::Day));
    assert_eq!(reason(Scalar::DateTime, "2021-01-11T25:00:00Z"), Some(Reason::Hour));
    assert_eq!(reason(Scalar::DateTime, "2021-01-11 12:30:24Z"), Some(Reason::Format));
    assert_eq!(reason(Scalar::DateTime, "2021-01-11t12:30:24z"), None);
    assert_eq!(reason(Scalar::DateTime, "2021-01-11Tt12:30:24Z"), Some(Reason::Format));
    // A malformed ending is reported as bad format, not as a missing offset.
    assert_eq!(reason(Scalar::DateTime, "2021-01-11T12:30:24 "), Some(Reason::Format));
}

#[test]
fn datetime_offsets_can_be_hours_alone_up_to_18_hours() {
    for offset in ["+05", "-05", "+18", "+18:00", "-18:00:00"] {
        let value = format!("2021-01-11T12:30:24{offset}");
        assert_eq!(reason(Scalar::DateTime, &value), None, "{offset}");
    }
    for offset in ["+18:01", "-18:00:01", "+19", "+19:00", "+23:59"] {
        let value = format!("2021-01-11T12:30:24{offset}");
        assert_eq!(reason(Scalar::DateTime, &value), Some(Reason::OffsetRange), "{offset}");
    }
    for offset in ["+24", "+24:00", "+05:60", "+0530", "+5"] {
        let value = format!("2021-01-11T12:30:24{offset}");
        assert_eq!(reason(Scalar::DateTime, &value), Some(Reason::Offset), "{offset}");
    }
}

#[test]
fn timestamp_is_integer_text_within_documented_limits() {
    for value in ["0", "-0", "1", "-1", "31556889864403199", "-31557014167219200"] {
        assert_eq!(reason(Scalar::Timestamp, value), None, "{value}");
    }
    for value in [
        "31556889864403200",
        "-31557014167219201",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775808",
        "-9223372036854775809",
    ] {
        assert_eq!(reason(Scalar::Timestamp, value), Some(Reason::TimestampRange), "{value}");
    }
    for value in ["", "-", "+1", "01", "1.0", "1e3", " 1", "1 ", "0x10", "abc"] {
        assert_eq!(reason(Scalar::Timestamp, value), Some(Reason::Format), "{value}");
    }
}

#[test]
fn lookalike_unicode_digits_are_rejected() {
    assert_eq!(reason(Scalar::Date, "٢٠٢١-٠١-١١"), Some(Reason::Format));
    assert_eq!(reason(Scalar::Time, "１２:００"), Some(Reason::Format));
    assert_eq!(reason(Scalar::Timestamp, "١"), Some(Reason::Format));
}

#[test]
fn errors_name_the_scalar_and_reason() {
    let error = validate(Scalar::DateTime, "2021-01-11T12:30:24").unwrap_err();
    assert_eq!(error.scalar(), Scalar::DateTime);
    assert_eq!(error.to_string(), "invalid AWSDateTime: time zone offset is required");
}
