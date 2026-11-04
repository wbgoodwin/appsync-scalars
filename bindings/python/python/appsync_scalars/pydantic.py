"""Pydantic field types for AppSync scalars.

Requires the ``pydantic`` extra: ``pip install appsync-scalars[pydantic]``.

Fields keep the exact ``str`` or ``int`` they were given. Python's ``datetime`` cannot hold
negative years and keeps only microseconds, so converting would break some valid values.
"""

from collections.abc import Callable
from typing import Annotated, Any

from pydantic import AfterValidator, Strict, WithJsonSchema
from pydantic_core import PydanticCustomError

from ._native import (
    validate_aws_date,
    validate_aws_datetime,
    validate_aws_time,
    validate_aws_timestamp,
)

__all__ = ["AWSDate", "AWSDateTime", "AWSTime", "AWSTimestamp"]


def _checked_by(error_type: str, validate: Callable[[Any], None]) -> AfterValidator:
    def check(value: Any) -> Any:
        try:
            validate(value)
        except ValueError as error:
            raise PydanticCustomError(error_type, "{reason}", {"reason": str(error)}) from None
        return value

    return AfterValidator(check)


# Strict stops Pydantic converting first: bytes to str, or "123" and True to int.
AWSDate = Annotated[
    str,
    Strict(),
    _checked_by("aws_date", validate_aws_date),
    WithJsonSchema({"type": "string", "format": "aws-date"}),
]

AWSTime = Annotated[
    str,
    Strict(),
    _checked_by("aws_time", validate_aws_time),
    WithJsonSchema({"type": "string", "format": "aws-time"}),
]

AWSDateTime = Annotated[
    str,
    Strict(),
    _checked_by("aws_datetime", validate_aws_datetime),
    WithJsonSchema({"type": "string", "format": "aws-datetime"}),
]

AWSTimestamp = Annotated[
    int,
    Strict(),
    _checked_by("aws_timestamp", validate_aws_timestamp),
    WithJsonSchema({"type": "integer", "format": "aws-timestamp"}),
]
