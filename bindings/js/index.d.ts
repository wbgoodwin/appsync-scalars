export type AppSyncScalar = "AWSDate" | "AWSTime" | "AWSDateTime" | "AWSTimestamp";

export class AppSyncScalarError extends Error {
  readonly scalar: AppSyncScalar;
  readonly reason: string;
  constructor(scalar: AppSyncScalar, reason: string);
}

// A value already of the right type gets a plain boolean, because a type guard would narrow the
// false branch to never. Anything else is narrowed when the check passes.
export function isAWSDate(value: string): boolean;
export function isAWSDate(value: unknown): value is string;
export function isAWSTime(value: string): boolean;
export function isAWSTime(value: unknown): value is string;
export function isAWSDateTime(value: string): boolean;
export function isAWSDateTime(value: unknown): value is string;
export function isAWSTimestamp(value: number | bigint): boolean;
export function isAWSTimestamp(value: unknown): value is number | bigint;

/** @throws {AppSyncScalarError} if the value is invalid. */
export function validateAWSDate(value: unknown): asserts value is string;
/** @throws {AppSyncScalarError} if the value is invalid. */
export function validateAWSTime(value: unknown): asserts value is string;
/** @throws {AppSyncScalarError} if the value is invalid. */
export function validateAWSDateTime(value: unknown): asserts value is string;
/** @throws {AppSyncScalarError} if the value is invalid. */
export function validateAWSTimestamp(value: unknown): asserts value is number | bigint;
