import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  AppSyncScalarError,
  isAWSDate,
  isAWSDateTime,
  isAWSTime,
  isAWSTimestamp,
  validateAWSDate,
  validateAWSDateTime,
  validateAWSTimestamp,
} from "../index.js";

const smokeCases = JSON.parse(
  readFileSync(new URL("../../../conformance/smoke.json", import.meta.url), "utf8"),
);
const checks = {
  AWSDate: isAWSDate,
  AWSTime: isAWSTime,
  AWSDateTime: isAWSDateTime,
  AWSTimestamp: isAWSTimestamp,
};

test("shared cases", () => {
  for (const { scalar, value, valid } of smokeCases) {
    assert.equal(checks[scalar](value), valid, `${scalar} ${JSON.stringify(value)}`);
  }
});

test("validate throws an error naming the scalar and reason", () => {
  assert.throws(
    () => validateAWSDateTime("2021-01-11T12:30:24"),
    (error) => {
      assert.ok(error instanceof AppSyncScalarError);
      assert.equal(error.scalar, "AWSDateTime");
      assert.equal(error.reason, "time zone offset is required");
      assert.equal(error.message, "invalid AWSDateTime: time zone offset is required");
      return true;
    },
  );
});

test("validate returns nothing for valid values", () => {
  assert.equal(validateAWSDate("2021-01-11"), undefined);
});

test("values that do not fit the buffer are invalid, not truncated and accepted", () => {
  assert.equal(isAWSDate("2021-01-11" + " ".repeat(5000)), false);
  assert.equal(isAWSDate("2021-01-11" + "é".repeat(5000)), false);
  // The start of this value would be valid if the module only saw what fit.
  assert.equal(isAWSTime("12:30" + "\u{1F600}".repeat(1000)), false);
});

test("a short value after a long one is not affected by leftover bytes", () => {
  assert.equal(isAWSDate("x".repeat(2000)), false);
  assert.equal(isAWSDate("2021-01-11"), true);
  assert.equal(isAWSDate("2021-01"), false);
});

test("non-ASCII text and lone surrogates are invalid", () => {
  assert.equal(isAWSDate("2021–01–11"), false);
  assert.equal(isAWSDate("2021-01-1\ud800"), false);
});

test("non-strings are invalid for text scalars", () => {
  for (const value of [undefined, null, 20210111, true, {}, [], new Date(0)]) {
    assert.equal(isAWSDateTime(value), false);
  }
  assert.throws(() => validateAWSDate(1), /invalid AWSDate: expected a string/);
});

test("timestamps must be integers within AppSync's limits", () => {
  assert.equal(isAWSTimestamp(0), true);
  assert.equal(isAWSTimestamp(-0), true);
  assert.equal(isAWSTimestamp(31556889864403199n), true);
  assert.equal(isAWSTimestamp(-31557014167219200n), true);
  assert.equal(isAWSTimestamp(31556889864403200n), false);
  assert.equal(isAWSTimestamp(-31557014167219201n), false);
  assert.equal(isAWSTimestamp(9223372036854775807n), false);
  assert.equal(isAWSTimestamp(1e21), false);
  for (const value of [1.5, NaN, Infinity, "1", true, null, undefined]) {
    assert.equal(isAWSTimestamp(value), false, String(value));
  }
  assert.throws(() => validateAWSTimestamp("1"), /expected an integer/);
  assert.throws(() => validateAWSTimestamp(9223372036854775808n), /out of range/);
});

test("the embedded module stays on short lines, so scanners don't read it as minified", () => {
  const source = readFileSync(new URL("../wasm.generated.js", import.meta.url), "utf8");
  for (const line of source.split("\n")) {
    assert.ok(line.length <= 100, `a line has ${line.length} characters`);
  }
});
