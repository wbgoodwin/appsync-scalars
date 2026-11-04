from datetime import datetime, timezone

import pytest
from pydantic import BaseModel, TypeAdapter, ValidationError

from appsync_scalars.pydantic import AWSDate, AWSDateTime, AWSTime, AWSTimestamp


class Event(BaseModel):
    day: AWSDate
    at: AWSTime | None = None
    starts_at: AWSDateTime
    created: AWSTimestamp


VALID = {
    "day": "1970-01-01+05:30",
    "at": "12:30",
    "starts_at": "2021-01-11T12:30:24.123456789Z",
    "created": 1610368224,
}


def error_for(field, value):
    with pytest.raises(ValidationError) as caught:
        Event(**{**VALID, field: value})
    errors = caught.value.errors()
    assert len(errors) == 1
    assert errors[0]["loc"] == (field,)
    return errors[0]


def test_valid_values_keep_their_exact_form():
    event = Event(**VALID)
    assert event.model_dump() == VALID
    assert isinstance(event.starts_at, str)


def test_values_python_datetime_cannot_hold_are_accepted_unchanged():
    negative_year = "-2017-01-01T00:00Z"
    assert TypeAdapter(AWSDateTime).validate_python(negative_year) == negative_year

    nanoseconds = "2021-01-11T12:30:24.123456789+00:00"
    assert TypeAdapter(AWSDateTime).validate_python(nanoseconds) == nanoseconds


def test_invalid_values_report_a_stable_error_type_and_reason():
    error = error_for("starts_at", "2021-01-11T12:30:24")
    assert error["type"] == "aws_datetime"
    assert error["msg"] == "invalid AWSDateTime: time zone offset is required"

    error = error_for("day", "2021-02-29")
    assert error["type"] == "aws_date"
    assert error["msg"] == "invalid AWSDate: day is out of range for the month"


def test_json_input_is_checked_the_same_way():
    payload = '{"day":"2021-01-11","starts_at":"2021-01-11T12:30:24","created":1}'
    with pytest.raises(ValidationError, match="time zone offset is required"):
        Event.model_validate_json(payload)


def test_optional_and_nested_types_work():
    assert Event(**{**VALID, "at": None}).at is None
    assert TypeAdapter(list[AWSDate]).validate_python(["2021-01-11"]) == ["2021-01-11"]
    with pytest.raises(ValidationError):
        TypeAdapter(list[AWSDate]).validate_python(["2021-13-01"])


@pytest.mark.parametrize("value", ["1610368224", 1.0, True])
def test_timestamp_is_not_coerced(value):
    assert error_for("created", value)["type"] == "int_type"


def test_timestamp_range_is_enforced():
    assert error_for("created", 2**63)["type"] == "aws_timestamp"


@pytest.mark.parametrize(
    "value", [b"2021-01-11T12:30:24Z", 1610368224, datetime(2021, 1, 11, tzinfo=timezone.utc)]
)
def test_strings_are_not_coerced(value):
    assert error_for("starts_at", value)["type"] == "string_type"


def test_json_schema_marks_the_scalar():
    properties = Event.model_json_schema()["properties"]
    assert properties["starts_at"]["format"] == "aws-datetime"
    assert properties["created"]["format"] == "aws-timestamp"
