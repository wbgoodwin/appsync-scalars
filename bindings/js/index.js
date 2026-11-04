import { WASM_BASE64 } from "./wasm.generated.js";

const SCALAR_CODES = { AWSDate: 0, AWSTime: 1, AWSDateTime: 2, AWSTimestamp: 3 };

export class AppSyncScalarError extends Error {
  constructor(scalar, reason) {
    super(`invalid ${scalar}: ${reason}`);
    this.name = "AppSyncScalarError";
    this.scalar = scalar;
    this.reason = reason;
  }
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();
let module;

// Loads on first use, so importing the package costs nothing until a value is checked.
function load() {
  if (!module) {
    const bytes = Uint8Array.from(atob(WASM_BASE64), (char) => char.charCodeAt(0));
    const { exports } = new WebAssembly.Instance(new WebAssembly.Module(bytes));
    module = {
      exports,
      inputPtr: exports.appsync_input_ptr(),
      inputCapacity: exports.appsync_input_capacity(),
    };
  }
  return module;
}

/** Returns null if `text` is valid, or the reason it is not. */
function reasonFor(scalar, text) {
  const { exports, inputPtr, inputCapacity } = load();
  const input = new Uint8Array(exports.memory.buffer, inputPtr, inputCapacity);
  const { read, written } = encoder.encodeInto(text, input);
  // Text that did not fit is reported as too long, which the module always rejects.
  const length = read < text.length ? inputCapacity + 1 : written;
  const code = exports.appsync_validate(SCALAR_CODES[scalar], length);
  if (code === 0) {
    return null;
  }
  const message = new Uint8Array(
    exports.memory.buffer,
    exports.appsync_reason_ptr(code),
    exports.appsync_reason_len(code),
  );
  return decoder.decode(message);
}

function checkText(scalar, value) {
  return typeof value === "string" ? reasonFor(scalar, value) : "expected a string";
}

function checkTimestamp(value) {
  if (typeof value === "bigint" || Number.isInteger(value)) {
    return reasonFor("AWSTimestamp", String(value));
  }
  return "expected an integer";
}

function throwIfInvalid(scalar, reason) {
  if (reason !== null) {
    throw new AppSyncScalarError(scalar, reason);
  }
}

export function isAWSDate(value) {
  return checkText("AWSDate", value) === null;
}

export function isAWSTime(value) {
  return checkText("AWSTime", value) === null;
}

export function isAWSDateTime(value) {
  return checkText("AWSDateTime", value) === null;
}

export function isAWSTimestamp(value) {
  return checkTimestamp(value) === null;
}

export function validateAWSDate(value) {
  throwIfInvalid("AWSDate", checkText("AWSDate", value));
}

export function validateAWSTime(value) {
  throwIfInvalid("AWSTime", checkText("AWSTime", value));
}

export function validateAWSDateTime(value) {
  throwIfInvalid("AWSDateTime", checkText("AWSDateTime", value));
}

export function validateAWSTimestamp(value) {
  throwIfInvalid("AWSTimestamp", checkTimestamp(value));
}
