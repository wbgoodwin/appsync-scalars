//! Feeds the validator random and damaged values, to catch panics the written tests never thought
//! of. A panic matters more outside Rust, where it can take down the caller's whole process.
//!
//! Each run uses a new seed. To replay a failure, set `APPSYNC_FUZZ_SEED` to the seed it reports.

use std::panic::catch_unwind;
use std::time::{SystemTime, UNIX_EPOCH};

use appsync_scalars::{Scalar, is_valid, validate};

const ROUNDS: usize = 200_000;
const SCALARS: [Scalar; 4] = [Scalar::Date, Scalar::Time, Scalar::DateTime, Scalar::Timestamp];

const VALID: &[&str] = &[
    "1970-01-01",
    "-2017-05-01",
    "+12021-01-11Z",
    "1970-01-01+05:30:15",
    "12:30",
    "12:00:34.",
    "12:30:24.123456789-07:00",
    "2021-01-11T12:30Z",
    "2021-01-11t12:30:24.5+05",
    "+999999999-12-31T23:59:59.999999999-18:00",
    "1610368224",
    "-31557014167219200",
];

const PIECES: &[&str] = &[
    "0", "1", "9", "00", "12", "59", "60", "99", "-", "+", ":", ".", "T", "t", "Z", "z", " ", "\n",
    "\0", "2021-01-11", "12:30:24", "+05:30", "9223372036854775807", "99999999999", "é", "٣", "😀",
];

/// xorshift64*: small, and all this needs is a repeatable stream of numbers.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next() % limit as u64) as usize
    }

    fn pick<'a>(&mut self, options: &[&'a str]) -> &'a str {
        options[self.below(options.len())]
    }
}

fn joined_pieces(random: &mut Random) -> String {
    (0..random.below(12)).map(|_| random.pick(PIECES)).collect()
}

fn damaged_value(random: &mut Random) -> String {
    let mut chars: Vec<char> = random.pick(VALID).chars().collect();
    for _ in 0..=random.below(4) {
        let position = random.below(chars.len() + 1);
        match random.below(3) {
            0 if position < chars.len() => {
                chars.remove(position);
            }
            1 if position < chars.len() => chars.truncate(position),
            _ => {
                let piece = random.pick(PIECES);
                chars.splice(position..position, piece.chars());
            }
        }
    }
    chars.into_iter().collect()
}

fn random_text(random: &mut Random) -> String {
    (0..random.below(40))
        .filter_map(|_| char::from_u32((random.next() % 0x11_0000) as u32))
        .collect()
}

fn check(text: &str) {
    for scalar in SCALARS {
        let result = validate(scalar, text);
        assert_eq!(is_valid(scalar, text), result.is_ok());
        if let Err(error) = result {
            assert_eq!(error.scalar(), scalar);
            assert!(error.to_string().starts_with("invalid "));
        }
    }
}

#[test]
fn random_values_never_panic() {
    let seed = std::env::var("APPSYNC_FUZZ_SEED")
        .ok()
        .and_then(|seed| seed.parse().ok())
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 | 1);
    let mut random = Random(seed);

    for _ in 0..ROUNDS {
        let text = match random.below(3) {
            0 => joined_pieces(&mut random),
            1 => damaged_value(&mut random),
            _ => random_text(&mut random),
        };
        if catch_unwind(|| check(&text)).is_err() {
            panic!("seed {seed} failed on {text:?}");
        }
    }
}

#[test]
fn very_long_fields_never_panic() {
    for length in [1, 4, 9, 10, 11, 19, 20, 21, 39, 40, 100, 5000] {
        let digits = "9".repeat(length);
        for text in [
            digits.clone(),
            format!("-{digits}"),
            format!("+{digits}-01-11"),
            format!("-{digits}-01-11T12:30Z"),
            format!("{digits}-{digits}-{digits}"),
            format!("12:30:24.{digits}"),
            format!("{digits}:{digits}:{digits}"),
            format!("12:30+{digits}"),
            format!("12:30+{digits}:{digits}:{digits}"),
        ] {
            check(&text);
        }
    }
}
