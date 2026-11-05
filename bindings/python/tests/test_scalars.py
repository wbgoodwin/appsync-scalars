import enum
import json
from pathlib import Path

import pytest

import appsync_scalars as scalars

SMOKE_CASES = json.loads(
    (Path(__file__).parents[3] / "conformance" / "smoke.json").read_text(encoding="utf-8")
)

CHECKS = {
    "AWSDate": scalars.is_aws_date,
    "AWSTime": scalars.is_aws_time,
    "AWSDateTime": scalars.is_aws_datetime,
    "AWSTimestamp": scalars.is_aws_timestamp,
}


@pytest.mark.parametrize("case", SMOKE_CASES, ids=lambda c: f"{c['scalar']}:{c['value']}")
def test_shared_cases(case):
    assert CHECKS[case["scalar"]](case["value"]) is case["valid"]


def test_validate_raises_with_the_reason():
    with pytest.raises(ValueError, match="invalid AWSDateTime: time zone offset is required"):
        scalars.validate_aws_datetime("2021-01-11T12:30:24")


def test_validate_returns_none_for_valid_values():
    assert scalars.validate_aws_date("2021-01-11") is None


@pytest.mark.parametrize("value", [True, False, 1.0, 1.5, "1", None, [], {}])
def test_timestamp_needs_a_real_integer(value):
    assert scalars.is_aws_timestamp(value) is False
    with pytest.raises(ValueError, match="expected an integer"):
        scalars.validate_aws_timestamp(value)


def test_timestamp_range_handles_big_python_integers():
    assert scalars.is_aws_timestamp(31556889864403199) is True
    assert scalars.is_aws_timestamp(-31557014167219200) is True
    assert scalars.is_aws_timestamp(31556889864403200) is False
    assert scalars.is_aws_timestamp(-31557014167219201) is False
    assert scalars.is_aws_timestamp(2**63 - 1) is False
    assert scalars.is_aws_timestamp(2**63) is False
    assert scalars.is_aws_timestamp(10**100) is False


def test_timestamp_far_too_large_to_print_is_out_of_range():
    # Python refuses to print an int this long, so its digits can't be checked as text.
    with pytest.raises(ValueError, match="invalid AWSTimestamp: timestamp is out of range"):
        scalars.validate_aws_timestamp(10**5000)


class LoudInt(int):
    def __str__(self):
        return "5"

    def __repr__(self):
        return "5"

    def __index__(self):
        return 5


class Five(enum.IntEnum):
    FIVE = 5


def test_int_subclasses_are_judged_by_their_value():
    # A subclass's own text doesn't change the verdict, on any Python version.
    assert scalars.is_aws_timestamp(LoudInt(10**30)) is False
    assert scalars.is_aws_timestamp(LoudInt(7)) is True
    assert scalars.is_aws_timestamp(Five.FIVE) is True


def test_lone_surrogates_are_invalid_rather_than_an_encoding_error():
    assert scalars.is_aws_date("2021-01-1\ud800") is False
    with pytest.raises(ValueError, match="format not recognized"):
        scalars.validate_aws_date("\ud800")


def test_non_strings_are_a_type_error_for_text_scalars():
    with pytest.raises(TypeError):
        scalars.is_aws_date(20210111)
