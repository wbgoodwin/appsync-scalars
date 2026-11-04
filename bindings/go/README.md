# appsync-scalars for Go

Checks whether a value is a valid AWS AppSync `AWSDate`, `AWSTime`, `AWSDateTime` or `AWSTimestamp`, using rules recorded from a live AppSync API. Use it to catch a bad value before AppSync rejects the whole request.

Also available for [Python](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/python/README.md), [JavaScript](https://github.com/wbgoodwin/appsync-scalars/blob/main/bindings/js/README.md) and [Rust](https://github.com/wbgoodwin/appsync-scalars/blob/main/crates/appsync-scalars/README.md).

## Install

```bash
go get github.com/wbgoodwin/appsync-scalars/bindings/go
```

It needs Go 1.25 or later. It doesn't use cgo, so it needs no C toolchain and cross-compiles normally.

## Usage

```go
package main

import (
	"errors"
	"fmt"
	"strconv"

	appsyncscalars "github.com/wbgoodwin/appsync-scalars/bindings/go"
)

func main() {
	fmt.Println(appsyncscalars.IsAWSDate("1970-01-01+05:30")) // true
	fmt.Println(appsyncscalars.IsAWSDate("2021-02-29"))       // false

	err := appsyncscalars.ValidateAWSDateTime("2021-01-11T12:30:24")
	var invalid *appsyncscalars.Error
	if errors.As(err, &invalid) {
		fmt.Println(invalid.Scalar) // AWSDateTime
		fmt.Println(invalid.Reason) // time zone offset is required
	}

	// AWSTimestamp functions take the integer's text.
	fmt.Println(appsyncscalars.IsAWSTimestamp(strconv.FormatInt(1610368224, 10))) // true
}
```

Each scalar has two functions, and all of them take a `string`.

| Scalar | Returns a `bool` | Returns an `error` |
|---|---|---|
| `AWSDate` | `IsAWSDate` | `ValidateAWSDate` |
| `AWSTime` | `IsAWSTime` | `ValidateAWSTime` |
| `AWSDateTime` | `IsAWSDateTime` | `ValidateAWSDateTime` |
| `AWSTimestamp` | `IsAWSTimestamp` | `ValidateAWSTimestamp` |

The `Validate` functions return `nil` or an `*Error`, with `Scalar` and `Reason` fields.

## How it runs

The validator is written in Rust and embedded as WebAssembly, which runs through [wazero](https://github.com/tetratelabs/wazero), a runtime written in Go. That has a few practical effects, measured on an Apple M4:

- **Binary size:** a small program grew by about 4.5 MB.
- **First call:** the module loads on first use, which takes 2 to 3 ms.
- **Later calls:** a check takes under 150 ns. A valid value doesn't allocate, and an invalid one allocates to build its error.
- **Concurrency:** all functions are safe to call from any goroutine, but checks run one at a time, so heavy parallel use will contend on a lock.

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
