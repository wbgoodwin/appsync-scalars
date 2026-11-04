// Package appsyncscalars validates AWS AppSync date and time scalar values.
//
// It runs the library's Rust validator as WebAssembly through wazero, a WebAssembly runtime
// written in Go, so it needs no cgo or C toolchain. The module loads on first use, and all
// functions are safe for concurrent use.
package appsyncscalars

import (
	"context"
	_ "embed"
	"fmt"
	"sync"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

//go:embed appsync_scalars.wasm
var wasmModule []byte

const (
	scalarDate uint64 = iota
	scalarTime
	scalarDateTime
	scalarTimestamp
)

var scalarNames = [...]string{"AWSDate", "AWSTime", "AWSDateTime", "AWSTimestamp"}

// Error reports why a value is invalid.
type Error struct {
	// Scalar is the AppSync type name, such as "AWSDateTime".
	Scalar string
	// Reason describes what is wrong with the value.
	Reason string
}

func (e *Error) Error() string {
	return "invalid " + e.Scalar + ": " + e.Reason
}

// IsAWSDate reports whether value is a valid AWSDate.
func IsAWSDate(value string) bool { return ValidateAWSDate(value) == nil }

// IsAWSTime reports whether value is a valid AWSTime.
func IsAWSTime(value string) bool { return ValidateAWSTime(value) == nil }

// IsAWSDateTime reports whether value is a valid AWSDateTime.
func IsAWSDateTime(value string) bool { return ValidateAWSDateTime(value) == nil }

// IsAWSTimestamp reports whether value is the text of a valid AWSTimestamp integer.
func IsAWSTimestamp(value string) bool { return ValidateAWSTimestamp(value) == nil }

// ValidateAWSDate returns an *Error if value is not a valid AWSDate.
func ValidateAWSDate(value string) error { return engine().check(scalarDate, value) }

// ValidateAWSTime returns an *Error if value is not a valid AWSTime.
func ValidateAWSTime(value string) error { return engine().check(scalarTime, value) }

// ValidateAWSDateTime returns an *Error if value is not a valid AWSDateTime.
func ValidateAWSDateTime(value string) error { return engine().check(scalarDateTime, value) }

// ValidateAWSTimestamp returns an *Error if value is not the text of a valid AWSTimestamp integer.
func ValidateAWSTimestamp(value string) error { return engine().check(scalarTimestamp, value) }

// A failure here means the embedded module is broken, which no caller can recover from.
var engine = sync.OnceValue(func() *validator {
	v, err := newValidator(context.Background())
	if err != nil {
		panic("appsyncscalars: loading embedded module: " + err.Error())
	}
	return v
})

type validator struct {
	ctx           context.Context
	memory        api.Memory
	validate      api.Function
	reasonPtr     api.Function
	reasonLen     api.Function
	inputPtr      uint32
	inputCapacity int

	// An instance handles one call at a time. mu guards the input buffer and stack.
	mu    sync.Mutex
	stack [2]uint64
}

func newValidator(ctx context.Context) (*validator, error) {
	runtime := wazero.NewRuntime(ctx)
	instance, err := runtime.Instantiate(ctx, wasmModule)
	if err != nil {
		return nil, err
	}
	inputPtr, err := instance.ExportedFunction("appsync_input_ptr").Call(ctx)
	if err != nil {
		return nil, err
	}
	inputCapacity, err := instance.ExportedFunction("appsync_input_capacity").Call(ctx)
	if err != nil {
		return nil, err
	}
	return &validator{
		ctx:           ctx,
		memory:        instance.Memory(),
		validate:      instance.ExportedFunction("appsync_validate"),
		reasonPtr:     instance.ExportedFunction("appsync_reason_ptr"),
		reasonLen:     instance.ExportedFunction("appsync_reason_len"),
		inputPtr:      api.DecodeU32(inputPtr[0]),
		inputCapacity: int(api.DecodeU32(inputCapacity[0])),
	}, nil
}

func (v *validator) check(scalar uint64, value string) error {
	v.mu.Lock()
	defer v.mu.Unlock()

	// A value that does not fit is reported as too long, which the module always rejects.
	length := min(len(value), v.inputCapacity+1)
	if length <= v.inputCapacity && !v.memory.WriteString(v.inputPtr, value) {
		panic("appsyncscalars: input buffer is outside module memory")
	}

	v.stack[0], v.stack[1] = scalar, api.EncodeU32(uint32(length))
	if err := v.validate.CallWithStack(v.ctx, v.stack[:]); err != nil {
		panic("appsyncscalars: calling embedded module: " + err.Error())
	}
	code := v.stack[0]
	if code == 0 {
		return nil
	}
	return &Error{Scalar: scalarNames[scalar], Reason: v.reason(code)}
}

func (v *validator) reason(code uint64) string {
	ptr, err := v.reasonPtr.Call(v.ctx, code)
	if err != nil {
		panic("appsyncscalars: calling embedded module: " + err.Error())
	}
	length, err := v.reasonLen.Call(v.ctx, code)
	if err != nil {
		panic("appsyncscalars: calling embedded module: " + err.Error())
	}
	message, ok := v.memory.Read(api.DecodeU32(ptr[0]), api.DecodeU32(length[0]))
	if !ok {
		panic(fmt.Sprintf("appsyncscalars: reason code %d is outside module memory", code))
	}
	return string(message)
}
