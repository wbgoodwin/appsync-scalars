// Checks an installed appsync-scalars against the shared cases and recorded AppSync results.
//
// Usage: acceptance <conformance directory>
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	appsyncscalars "github.com/wbgoodwin/appsync-scalars/bindings/go"
)

var checks = map[string]func(string) bool{
	"AWSDate":      appsyncscalars.IsAWSDate,
	"AWSTime":      appsyncscalars.IsAWSTime,
	"AWSDateTime":  appsyncscalars.IsAWSDateTime,
	"AWSTimestamp": appsyncscalars.IsAWSTimestamp,
}

type check struct {
	scalar string
	text   string
	valid  bool
}

// asText returns the form a value takes in the Go API, or false if it has no way to express it.
func asText(scalar string, raw json.RawMessage) (string, bool) {
	isString := raw[0] == '"'
	isNumber := raw[0] == '-' || (raw[0] >= '0' && raw[0] <= '9')
	if scalar == "AWSTimestamp" {
		return string(raw), isNumber
	}
	if !isString {
		return "", false
	}
	var text string
	if err := json.Unmarshal(raw, &text); err != nil {
		panic(err)
	}
	return text, true
}

func load(path string, into any) {
	data, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	if err := json.Unmarshal(data, into); err != nil {
		panic(err)
	}
}

func main() {
	conformance := os.Args[1]
	var expected []check

	var smoke []struct {
		Scalar string          `json:"scalar"`
		Value  json.RawMessage `json:"value"`
		Valid  bool            `json:"valid"`
	}
	load(filepath.Join(conformance, "smoke.json"), &smoke)
	for _, c := range smoke {
		text, _ := asText(c.Scalar, c.Value)
		expected = append(expected, check{c.Scalar, text, c.Valid})
	}

	type outcome struct {
		Result string `json:"result"`
	}
	for _, stem := range []string{"aws-date", "aws-time", "aws-datetime", "aws-timestamp"} {
		var vectors struct {
			Scalar string `json:"scalar"`
			Cases  []struct {
				Value         json.RawMessage `json:"value"`
				InputVariable outcome         `json:"input_variable"`
				Output        outcome         `json:"output"`
			} `json:"cases"`
		}
		load(filepath.Join(conformance, "vectors", stem+".json"), &vectors)
		if len(vectors.Cases) == 0 {
			panic(stem + ".json has no recorded cases")
		}
		for _, c := range vectors.Cases {
			text, ok := asText(vectors.Scalar, c.Value)
			if !ok {
				continue
			}
			for _, path := range []outcome{c.InputVariable, c.Output} {
				expected = append(expected, check{vectors.Scalar, text, path.Result == "accepted"})
			}
		}
	}

	if len(expected) == 0 {
		panic("no cases to check")
	}
	matched := 0
	for _, c := range expected {
		if checks[c.scalar](c.text) == c.valid {
			matched++
		} else {
			fmt.Printf("%s %q: expected valid = %v\n", c.scalar, c.text, c.valid)
		}
	}
	fmt.Printf("Go: %d of %d checks match\n", matched, len(expected))
	if matched != len(expected) {
		os.Exit(1)
	}
}
