//! Validates the date and time scalars AWS AppSync defines.
//!
//! ```
//! use appsync_scalars::{Reason, is_aws_date, validate_aws_datetime};
//!
//! assert!(is_aws_date("1970-01-01+05:30"));
//! assert!(!is_aws_date("2021-02-29"));
//!
//! let error = validate_aws_datetime("2021-01-11T12:30:24").unwrap_err();
//! assert_eq!(error.reason(), Reason::OffsetRequired);
//! ```
//!
//! These rules follow results recorded from a live API in `conformance/`, not AppSync's written
//! descriptions, because AppSync accepts more than they say. It takes a lowercase `z` or `t`, an
//! empty fraction (`12:00:34.`), and signed years of up to ten digits. `AWSDateTime` also has its
//! own offset rule: it takes `+05` but nothing beyond 18 hours.

#![no_std]

use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scalar {
    Date,
    Time,
    DateTime,
    Timestamp,
}

impl Scalar {
    pub const fn name(self) -> &'static str {
        match self {
            Scalar::Date => "AWSDate",
            Scalar::Time => "AWSTime",
            Scalar::DateTime => "AWSDateTime",
            Scalar::Timestamp => "AWSTimestamp",
        }
    }
}

/// Why a value was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Reason {
    Format,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Fraction,
    Offset,
    OffsetRequired,
    TimestampRange,
    Year,
    OffsetRange,
}

impl Reason {
    /// Every reason. The WebAssembly module's reason codes are positions in this list, so new
    /// reasons go at the end.
    pub const ALL: [Reason; 12] = [
        Reason::Format,
        Reason::Month,
        Reason::Day,
        Reason::Hour,
        Reason::Minute,
        Reason::Second,
        Reason::Fraction,
        Reason::Offset,
        Reason::OffsetRequired,
        Reason::TimestampRange,
        Reason::Year,
        Reason::OffsetRange,
    ];

    pub const fn message(self) -> &'static str {
        match self {
            Reason::Format => "format not recognized",
            Reason::Month => "month is out of range",
            Reason::Day => "day is out of range for the month",
            Reason::Hour => "hour is out of range",
            Reason::Minute => "minute is out of range",
            Reason::Second => "second is out of range",
            Reason::Fraction => "fractional seconds can have at most 9 digits",
            Reason::Offset => "time zone offset must be Z, +hh:mm or +hh:mm:ss",
            Reason::OffsetRequired => "time zone offset is required",
            Reason::TimestampRange => "timestamp is out of range",
            Reason::Year => "year is out of range",
            Reason::OffsetRange => "time zone offset can be at most 18 hours",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    scalar: Scalar,
    reason: Reason,
}

impl Error {
    pub fn scalar(&self) -> Scalar {
        self.scalar
    }

    pub fn reason(&self) -> Reason {
        self.reason
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}: {}", self.scalar.name(), self.reason.message())
    }
}

impl core::error::Error for Error {}

/// The largest year either way, the limit of the Java date types AppSync appears to use.
const MAX_YEAR: u64 = 999_999_999;

/// The largest `AWSDateTime` offset. `AWSDate` and `AWSTime` have no such limit.
const MAX_DATETIME_OFFSET_SECONDS: u64 = 18 * 60 * 60;

/// The limits AppSync's description of `AWSTimestamp` gives, which the recorded results confirm.
const MIN_TIMESTAMP: i64 = -31_557_014_167_219_200;
const MAX_TIMESTAMP: i64 = 31_556_889_864_403_199;

/// Checks `value` as text. `AWSTimestamp` values are the digits of the integer.
pub fn validate(scalar: Scalar, value: &str) -> Result<(), Error> {
    let result = match scalar {
        Scalar::Date => check_date(value),
        Scalar::Time => check_time(value),
        Scalar::DateTime => check_datetime(value),
        Scalar::Timestamp => check_timestamp(value),
    };
    result.map_err(|reason| Error { scalar, reason })
}

pub fn is_valid(scalar: Scalar, value: &str) -> bool {
    validate(scalar, value).is_ok()
}

pub fn validate_aws_date(value: &str) -> Result<(), Error> {
    validate(Scalar::Date, value)
}

pub fn validate_aws_time(value: &str) -> Result<(), Error> {
    validate(Scalar::Time, value)
}

pub fn validate_aws_datetime(value: &str) -> Result<(), Error> {
    validate(Scalar::DateTime, value)
}

pub fn validate_aws_timestamp(value: &str) -> Result<(), Error> {
    validate(Scalar::Timestamp, value)
}

pub fn is_aws_date(value: &str) -> bool {
    is_valid(Scalar::Date, value)
}

pub fn is_aws_time(value: &str) -> bool {
    is_valid(Scalar::Time, value)
}

pub fn is_aws_datetime(value: &str) -> bool {
    is_valid(Scalar::DateTime, value)
}

pub fn is_aws_timestamp(value: &str) -> bool {
    is_valid(Scalar::Timestamp, value)
}

fn check_date(value: &str) -> Result<(), Reason> {
    let mut cursor = Cursor::new(value);
    calendar_date(&mut cursor)?;
    offset(&mut cursor, false)?;
    cursor.finish()
}

fn check_time(value: &str) -> Result<(), Reason> {
    let mut cursor = Cursor::new(value);
    clock_time(&mut cursor)?;
    offset(&mut cursor, false)?;
    cursor.finish()
}

fn check_datetime(value: &str) -> Result<(), Reason> {
    let mut cursor = Cursor::new(value);
    calendar_date(&mut cursor)?;
    if !cursor.eat(b'T') && !cursor.eat(b't') {
        return Err(Reason::Format);
    }
    clock_time(&mut cursor)?;
    let offset = offset(&mut cursor, true)?;
    cursor.finish()?;
    match offset {
        None => Err(Reason::OffsetRequired),
        Some(seconds) if seconds > MAX_DATETIME_OFFSET_SECONDS => Err(Reason::OffsetRange),
        Some(_) => Ok(()),
    }
}

fn check_timestamp(value: &str) -> Result<(), Reason> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let is_integer = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'));
    if !is_integer {
        return Err(Reason::Format);
    }
    match value.parse::<i64>() {
        Ok(timestamp) if (MIN_TIMESTAMP..=MAX_TIMESTAMP).contains(&timestamp) => Ok(()),
        _ => Err(Reason::TimestampRange),
    }
}

/// Reads a year and returns its size. Four digits need no sign and may have `-`, but not `+`.
/// More digits need a sign, up to ten including leading zeros. `-0000` is not a year.
fn year(cursor: &mut Cursor<'_>) -> Result<u64, Reason> {
    let negative = cursor.eat(b'-');
    let positive = !negative && cursor.eat(b'+');
    let width = cursor.leading_digits();
    let allowed = if negative {
        4..=10
    } else if positive {
        5..=10
    } else {
        4..=4
    };
    if !allowed.contains(&width) {
        return Err(Reason::Format);
    }
    let year = cursor.number(width).ok_or(Reason::Format)?;
    if negative && year == 0 {
        return Err(Reason::Format);
    }
    if year > MAX_YEAR {
        return Err(Reason::Year);
    }
    Ok(year)
}

/// Reads a year, then `-MM-DD`, and checks the date exists on the proleptic Gregorian calendar.
fn calendar_date(cursor: &mut Cursor<'_>) -> Result<(), Reason> {
    let year = year(cursor)?;
    cursor.expect(b'-')?;
    let month = cursor.number(2).ok_or(Reason::Format)?;
    cursor.expect(b'-')?;
    let day = cursor.number(2).ok_or(Reason::Format)?;
    if !(1..=12).contains(&month) {
        return Err(Reason::Month);
    }
    if !(1..=days_in_month(year, month)).contains(&day) {
        return Err(Reason::Day);
    }
    Ok(())
}

/// A negative year has the same leap cycle as its positive counterpart, so the sign is unneeded.
fn days_in_month(year: u64, month: u64) -> u64 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Reads `hh:mm[:ss[.fffffffff]]`. The fraction may be empty, leaving a bare `.`.
fn clock_time(cursor: &mut Cursor<'_>) -> Result<(), Reason> {
    let hour = cursor.number(2).ok_or(Reason::Format)?;
    cursor.expect(b':')?;
    let minute = cursor.number(2).ok_or(Reason::Format)?;
    if hour > 23 {
        return Err(Reason::Hour);
    }
    if minute > 59 {
        return Err(Reason::Minute);
    }
    if !cursor.eat(b':') {
        return Ok(());
    }
    let second = cursor.number(2).ok_or(Reason::Format)?;
    if second > 59 {
        return Err(Reason::Second);
    }
    if cursor.eat(b'.') && cursor.digit_run() > 9 {
        return Err(Reason::Fraction);
    }
    Ok(())
}

/// Reads an optional `Z`, `z`, `+hh:mm` or `+hh:mm:ss`, and returns its size in seconds if one
/// was present. `hours_alone` also allows `+hh`.
fn offset(cursor: &mut Cursor<'_>, hours_alone: bool) -> Result<Option<u64>, Reason> {
    if cursor.eat(b'Z') || cursor.eat(b'z') {
        return Ok(Some(0));
    }
    if !cursor.eat(b'+') && !cursor.eat(b'-') {
        return Ok(None);
    }
    let hours = cursor.number(2).ok_or(Reason::Offset)?;
    let (mut minutes, mut seconds) = (0, 0);
    if cursor.eat(b':') {
        minutes = cursor.number(2).ok_or(Reason::Offset)?;
        if cursor.eat(b':') {
            seconds = cursor.number(2).ok_or(Reason::Offset)?;
        }
    } else if !hours_alone || cursor.leading_digits() > 0 {
        return Err(Reason::Offset);
    }
    if hours > 23 || minutes > 59 || seconds > 59 {
        return Err(Reason::Offset);
    }
    Ok(Some(hours * 3600 + minutes * 60 + seconds))
}

/// Every valid character is ASCII, so reading bytes also rejects lookalike Unicode digits.
struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn new(value: &'a str) -> Self {
        Cursor { rest: value.as_bytes() }
    }

    fn eat(&mut self, byte: u8) -> bool {
        match self.rest.split_first() {
            Some((&first, rest)) if first == byte => {
                self.rest = rest;
                true
            }
            _ => false,
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), Reason> {
        if self.eat(byte) { Ok(()) } else { Err(Reason::Format) }
    }

    /// Reads exactly `width` digits.
    fn number(&mut self, width: usize) -> Option<u64> {
        let digits = self.rest.get(..width)?;
        let mut number = 0;
        for &byte in digits {
            if !byte.is_ascii_digit() {
                return None;
            }
            number = number * 10 + u64::from(byte - b'0');
        }
        self.rest = &self.rest[width..];
        Some(number)
    }

    /// Counts the digits at the current position without reading them.
    fn leading_digits(&self) -> usize {
        self.rest.iter().take_while(|byte| byte.is_ascii_digit()).count()
    }

    /// Skips any digits and returns how many there were.
    fn digit_run(&mut self) -> usize {
        let count = self.leading_digits();
        self.rest = &self.rest[count..];
        count
    }

    fn finish(&self) -> Result<(), Reason> {
        if self.rest.is_empty() { Ok(()) } else { Err(Reason::Format) }
    }
}
