# appsync-scalars for Python

Checks whether a value is a valid AWS AppSync `AWSDate`, `AWSTime`, `AWSDateTime` or `AWSTimestamp`, using rules recorded from a live AppSync API. Use it to catch a bad value before AppSync rejects the whole request.

Also available for [JavaScript](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/js/README.md), [Rust](https://github.com/wbgoodwin/appsync-scalars/blob/main/crates/appsync-scalars/README.md) and [Go](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/go/README.md).

## Install

```bash
pip install appsync-scalars
```

It needs Python 3.10 or later and has no dependencies. Prebuilt wheels cover Linux (x64 and arm64, glibc 2.17 or later), macOS (Apple silicon and Intel) and Windows (x64). Anywhere else, including Alpine Linux, pip builds from source, which needs Rust.

## Usage

```python
from appsync_scalars import is_aws_date, validate_aws_datetime

is_aws_date("1970-01-01+05:30")  # True
is_aws_date("2021-02-29")  # False

try:
    validate_aws_datetime("2021-01-11T12:30:24")
except ValueError as error:
    print(error)  # invalid AWSDateTime: time zone offset is required
```

Each scalar has two functions. The `is_` function returns a `bool`, and the `validate_` function returns `None` or raises `ValueError` with the reason.

| Scalar | Takes | Returns a `bool` | Raises `ValueError` |
|---|---|---|---|
| `AWSDate` | `str` | `is_aws_date` | `validate_aws_date` |
| `AWSTime` | `str` | `is_aws_time` | `validate_aws_time` |
| `AWSDateTime` | `str` | `is_aws_datetime` | `validate_aws_datetime` |
| `AWSTimestamp` | `int` | `is_aws_timestamp` | `validate_aws_timestamp` |

Passing a non-string to a text scalar's functions raises `TypeError`. The `AWSTimestamp` functions treat anything but an `int` as invalid, including `True` and `"123"`.

## Pydantic

Field types for Pydantic v2 come with an optional extra:

```bash
pip install "appsync-scalars[pydantic]"
```

```python
from pydantic import BaseModel, ValidationError
from appsync_scalars.pydantic import AWSDate, AWSDateTime, AWSTimestamp

class Event(BaseModel):
    day: AWSDate
    starts_at: AWSDateTime
    created: AWSTimestamp

Event(day="2021-01-11", starts_at="2021-01-11T12:30:24Z", created=1610368224)

try:
    Event(day="2021-02-29", starts_at="2021-01-11T12:30:24Z", created=1610368224)
except ValidationError as error:
    print(error.errors()[0]["type"])  # aws_date
```

`AWSDate`, `AWSTime`, `AWSDateTime` and `AWSTimestamp` behave like this:

- **Values stay as given.** A field holds the exact `str` or `int` you passed, not a `datetime`, because `datetime` can't hold negative years and keeps only microseconds.
- **Nothing is converted.** Fields are strict, so `"123"` isn't accepted as an `AWSTimestamp`, and a `datetime` isn't accepted as an `AWSDateTime`.
- **Invalid values have stable error types.** They are `aws_date`, `aws_time`, `aws_datetime` and `aws_timestamp`, and the message gives the reason. A value of the wrong type gets Pydantic's own `string_type` or `int_type` instead.
- **JSON Schema shows the type.** Fields use a `format` of `aws-date`, `aws-time`, `aws-datetime` or `aws-timestamp`.

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
