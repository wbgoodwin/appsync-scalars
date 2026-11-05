"""Checks an installed appsync-scalars against the shared cases and recorded AppSync results.

Usage: python check.py <conformance directory>
"""

import json
import sys
from pathlib import Path

import appsync_scalars as scalars

CHECKS = {
    "AWSDate": scalars.is_aws_date,
    "AWSTime": scalars.is_aws_time,
    "AWSDateTime": scalars.is_aws_datetime,
    "AWSTimestamp": scalars.is_aws_timestamp,
}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def can_express(scalar, value):
    """Recordings include other JSON types, which the text scalars' functions refuse to take."""
    if scalar == "AWSTimestamp":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    return isinstance(value, str)


conformance = Path(sys.argv[1])
expected = [(c["scalar"], c["value"], c["valid"]) for c in load(conformance / "smoke.json")]
for stem in ("aws-date", "aws-time", "aws-datetime", "aws-timestamp"):
    vectors = load(conformance / "vectors" / f"{stem}.json")
    if not vectors["cases"]:
        sys.exit(f"{stem}.json has no recorded cases")
    for case in vectors["cases"]:
        if can_express(vectors["scalar"], case["value"]):
            for path in ("input_variable", "output"):
                accepted = case[path]["result"] == "accepted"
                expected.append((vectors["scalar"], case["value"], accepted))

if not expected:
    sys.exit("no cases to check")
mismatches = [case for case in expected if CHECKS[case[0]](case[1]) is not case[2]]
for scalar, value, valid in mismatches:
    print(f"{scalar} {ascii(value)}: expected valid = {valid}")
print(f"Python: {len(expected) - len(mismatches)} of {len(expected)} checks match")
sys.exit(1 if mismatches else 0)
