# appsync-scalars for Rust

Checks whether a value is a valid AWS AppSync `AWSDate`, `AWSTime`, `AWSDateTime` or `AWSTimestamp`, using rules recorded from a live AppSync API. Use it to catch a bad value before AppSync rejects the whole request.

Also available for [Python](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/python/README.md), [JavaScript](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/js/README.md) and [Go](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/go/README.md).

## Install

```bash
cargo add appsync-scalars
```

The crate is `no_std`, doesn't allocate and has no dependencies.

## Usage

```rust
use appsync_scalars::{Reason, is_aws_date, validate_aws_datetime};

assert!(is_aws_date("1970-01-01+05:30"));
assert!(!is_aws_date("2021-02-29"));

let error = validate_aws_datetime("2021-01-11T12:30:24").unwrap_err();
assert_eq!(error.reason(), Reason::OffsetRequired);
assert_eq!(error.to_string(), "invalid AWSDateTime: time zone offset is required");
```

Each scalar has two functions, and all of them take a `&str`.

| Scalar | Returns a `bool` | Returns a `Result<(), Error>` |
|---|---|---|
| `AWSDate` | `is_aws_date` | `validate_aws_date` |
| `AWSTime` | `is_aws_time` | `validate_aws_time` |
| `AWSDateTime` | `is_aws_datetime` | `validate_aws_datetime` |
| `AWSTimestamp` | `is_aws_timestamp` | `validate_aws_timestamp` |

To choose the scalar at run time, use `is_valid` or `validate`:

```rust
use appsync_scalars::{Scalar, is_valid};

assert!(is_valid(Scalar::Time, "12:30:24-07:00"));
```

## Timestamps

The `AWSTimestamp` functions take the integer's text, so format your integer first:

```rust
use appsync_scalars::is_aws_timestamp;

let seconds: i64 = 1610368224;
assert!(is_aws_timestamp(&seconds.to_string()));
assert!(!is_aws_timestamp("1.5"));
```

## Errors

`Error` implements `core::error::Error`. It has `scalar()`, which returns a `Scalar`, and `reason()`, which returns a `Reason`. `Reason::message()` gives the text. `Reason` is `#[non_exhaustive]`, so match it with a wildcard arm.

## What counts as valid

| Scalar | Valid examples |
|---|---|
| `AWSDate` | `1970-01-01`, `1970-01-01Z`, `1970-01-01+05:30`, `-2017-05-01` |
| `AWSTime` | `12:30`, `12:30:24.500`, `12:30:24-07:00` |
| `AWSDateTime` | `2021-01-11T12:30Z`, `1970-01-01T12:00:00.123456789+05:30` (the offset is required) |
| `AWSTimestamp` | `1610368224`, `-123123` (whole seconds since 1970) |

AppSync accepts and rejects some values its descriptions don't mention, and this library follows AppSync. See [what AppSync accepts](https://github.com/wbgoodwin/appsync-scalars#what-appsync-accepts).

## License

[MIT](https://github.com/wbgoodwin/appsync-scalars/blob/main/LICENSE)
