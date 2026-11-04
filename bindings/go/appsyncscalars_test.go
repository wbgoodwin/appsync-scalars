package appsyncscalars

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

type smokeCase struct {
	Scalar string          `json:"scalar"`
	Value  json.RawMessage `json:"value"`
	Valid  bool            `json:"valid"`
}

func (c smokeCase) text(t *testing.T) string {
	t.Helper()
	if c.Scalar == "AWSTimestamp" {
		return string(c.Value)
	}
	var text string
	if err := json.Unmarshal(c.Value, &text); err != nil {
		t.Fatal(err)
	}
	return text
}

func TestSharedCases(t *testing.T) {
	data, err := os.ReadFile(filepath.Join("..", "..", "conformance", "smoke.json"))
	if err != nil {
		t.Fatal(err)
	}
	var cases []smokeCase
	if err := json.Unmarshal(data, &cases); err != nil {
		t.Fatal(err)
	}
	checks := map[string]func(string) bool{
		"AWSDate":      IsAWSDate,
		"AWSTime":      IsAWSTime,
		"AWSDateTime":  IsAWSDateTime,
		"AWSTimestamp": IsAWSTimestamp,
	}
	for _, c := range cases {
		text := c.text(t)
		if got := checks[c.Scalar](text); got != c.Valid {
			t.Errorf("%s %q: got %v, want %v", c.Scalar, text, got, c.Valid)
		}
	}
}

func TestErrorNamesScalarAndReason(t *testing.T) {
	err := ValidateAWSDateTime("2021-01-11T12:30:24")
	var scalarErr *Error
	if !errors.As(err, &scalarErr) {
		t.Fatalf("got %T, want *Error", err)
	}
	if scalarErr.Scalar != "AWSDateTime" || scalarErr.Reason != "time zone offset is required" {
		t.Errorf("got %+v", scalarErr)
	}
	if want := "invalid AWSDateTime: time zone offset is required"; err.Error() != want {
		t.Errorf("got %q, want %q", err.Error(), want)
	}
}

func TestValidValueReturnsNil(t *testing.T) {
	if err := ValidateAWSDate("2021-01-11"); err != nil {
		t.Errorf("got %v", err)
	}
}

func TestOversizedValuesAreInvalid(t *testing.T) {
	// The start of each value would be valid if the module only saw what fit.
	for _, value := range []string{
		"2021-01-11" + strings.Repeat(" ", 5000),
		"2021-01-11" + strings.Repeat("é", 5000),
	} {
		if IsAWSDate(value) {
			t.Errorf("accepted a %d-byte value", len(value))
		}
	}
}

func TestShortValueAfterLongOneIsNotAffectedByLeftoverBytes(t *testing.T) {
	if IsAWSDate(strings.Repeat("x", 500)) {
		t.Error("accepted junk")
	}
	if !IsAWSDate("2021-01-11") {
		t.Error("rejected a valid date")
	}
	if IsAWSDate("2021-01") {
		t.Error("accepted a truncated date")
	}
}

func TestInvalidUTF8IsInvalid(t *testing.T) {
	if IsAWSDate("2021-01-1\xff") {
		t.Error("accepted invalid UTF-8")
	}
}

func TestTimestampText(t *testing.T) {
	for value, want := range map[string]bool{
		"0":                   true,
		"-1":                  true,
		"31556889864403199":   true,
		"-31557014167219200":  true,
		"31556889864403200":   false,
		"-31557014167219201":  false,
		"9223372036854775807": false,
		"1.5":                 false,
		"01":                  false,
		"":                    false,
	} {
		if got := IsAWSTimestamp(value); got != want {
			t.Errorf("%q: got %v, want %v", value, got, want)
		}
	}
}

func TestConcurrentUse(t *testing.T) {
	var wg sync.WaitGroup
	for worker := range 16 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			good := fmt.Sprintf("2021-01-%02d", worker+1)
			for range 2000 {
				if !IsAWSDate(good) {
					t.Errorf("rejected %s", good)
					return
				}
				if IsAWSDate("2021-13-01") {
					t.Error("accepted month 13")
					return
				}
			}
		}()
	}
	wg.Wait()
}

func BenchmarkValid(b *testing.B) {
	b.ReportAllocs()
	for b.Loop() {
		if !IsAWSDateTime("2021-01-11T12:30:24.123456789+05:30") {
			b.Fatal("rejected")
		}
	}
}

func BenchmarkInvalid(b *testing.B) {
	b.ReportAllocs()
	for b.Loop() {
		if IsAWSDateTime("2021-01-11T12:30:24") {
			b.Fatal("accepted")
		}
	}
}
