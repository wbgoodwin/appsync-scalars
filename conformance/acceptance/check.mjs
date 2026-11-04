// Checks an installed appsync-scalars against the shared cases and recorded AppSync results.
//
// Usage: node check.mjs <conformance directory>

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isAWSDate, isAWSDateTime, isAWSTime, isAWSTimestamp } from "appsync-scalars";

const checks = {
  AWSDate: isAWSDate,
  AWSTime: isAWSTime,
  AWSDateTime: isAWSDateTime,
  AWSTimestamp: isAWSTimestamp,
};
const conformance = process.argv[2];

// JSON.parse would round large integers, so each case's integer value is kept as text.
function loadVectors(stem) {
  const text = readFileSync(join(conformance, "vectors", `${stem}.json`), "utf8");
  return JSON.parse(text.replace(/"value": (-?\d+),$/gm, '"value": {"integer": "$1"},'));
}

// A JavaScript number can't tell 1.0 from 1, so timestamps written as decimals have no equivalent.
function toArgument(scalar, value) {
  if (scalar === "AWSTimestamp") {
    return value?.integer === undefined ? undefined : BigInt(value.integer);
  }
  return typeof value === "string" ? value : undefined;
}

const expected = JSON.parse(readFileSync(join(conformance, "smoke.json"), "utf8"));
for (const stem of ["aws-date", "aws-time", "aws-datetime", "aws-timestamp"]) {
  const { scalar, cases } = loadVectors(stem);
  for (const { value, input_variable, output } of cases) {
    const argument = toArgument(scalar, value);
    if (argument !== undefined) {
      for (const path of [input_variable, output]) {
        expected.push({ scalar, value: argument, valid: path.result === "accepted" });
      }
    }
  }
}

const mismatches = expected.filter(({ scalar, value, valid }) => checks[scalar](value) !== valid);
for (const { scalar, value, valid } of mismatches) {
  console.log(`${scalar} ${String(value)}: expected valid = ${valid}`);
}
console.log(`npm: ${expected.length - mismatches.length} of ${expected.length} checks match`);
process.exit(mismatches.length === 0 ? 0 : 1);
