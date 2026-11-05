// Compiled with tsc --noEmit in CI to check index.d.ts works the way people use it.
import { isAWSDate, isAWSTimestamp, validateAWSDate, validateAWSTimestamp } from "../index.js";

// A string that fails the check is still a string.
export function length(day: string): number {
  if (isAWSDate(day)) {
    return day.length;
  }
  return day.length;
}

export function seconds(value: number): number {
  if (isAWSTimestamp(value)) {
    return value;
  }
  return value + 0;
}

// Untrusted input is narrowed when the check passes.
export function fromInput(input: unknown): string | undefined {
  if (isAWSDate(input)) {
    const day: string = input;
    return day;
  }
  return undefined;
}

export function asserted(day: unknown, created: unknown): [string, number | bigint] {
  validateAWSDate(day);
  validateAWSTimestamp(created);
  return [day, created];
}
