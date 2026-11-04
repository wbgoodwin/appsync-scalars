# appsync-scalars for JavaScript and TypeScript

Checks whether a value is a valid AWS AppSync `AWSDate`, `AWSTime`, `AWSDateTime` or `AWSTimestamp`, using rules recorded from a live AppSync API. Use it to catch a bad value before AppSync rejects the whole request.

Also available for [Python](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/python/README.md), [Rust](https://github.com/wbgoodwin/appsync-scalars/blob/main/crates/appsync-scalars/README.md) and [Go](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/go/README.md).

## Install

```bash
npm install appsync-scalars
```

It's an ES module with no dependencies, TypeScript types included. The validator is WebAssembly embedded in the package, so there's no install script or platform-specific build. It's tested on Node.js 20, 24 and 26, but not in browsers or edge runtimes.

## Usage

```js
import { AppSyncScalarError, isAWSDate, validateAWSDateTime } from "appsync-scalars";

isAWSDate("1970-01-01+05:30"); // true
isAWSDate("2021-02-29"); // false

try {
  validateAWSDateTime("2021-01-11T12:30:24");
} catch (error) {
  if (error instanceof AppSyncScalarError) {
    console.log(error.message); // invalid AWSDateTime: time zone offset is required
    console.log(error.scalar); // AWSDateTime
    console.log(error.reason); // time zone offset is required
  }
}
```

Each scalar has two functions. The `is` function returns a boolean, and the `validate` function returns nothing or throws an `AppSyncScalarError`.

| Scalar | Takes | Returns a boolean | Throws |
|---|---|---|---|
| `AWSDate` | `string` | `isAWSDate` | `validateAWSDate` |
| `AWSTime` | `string` | `isAWSTime` | `validateAWSTime` |
| `AWSDateTime` | `string` | `isAWSDateTime` | `validateAWSDateTime` |
| `AWSTimestamp` | `number` or `bigint` | `isAWSTimestamp` | `validateAWSTimestamp` |

Any other type counts as invalid. Timestamps must be whole numbers, so `1.5` and `"123"` are invalid too.

## TypeScript

The functions accept `unknown` and narrow the type, so they work directly on untrusted input:

```ts
import { isAWSDate, validateAWSTimestamp } from "appsync-scalars";

function check(day: unknown, created: unknown) {
  if (isAWSDate(day)) {
    day.toUpperCase(); // day is a string here
  }
  validateAWSTimestamp(created);
  return created; // number | bigint
}
```

## Large timestamps

Use a `bigint` for a timestamp beyond `Number.MAX_SAFE_INTEGER`. A `number` that large has already been rounded before this library sees it, so the largest valid value only passes as a `bigint`: `31556889864403199n`.

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
