# AppSync conformance suite

AWS's documentation conflicts with itself on what the AppSync date and time scalars accept. This suite settles it by sending values to a real AppSync API and recording what happens. The recorded files in `vectors/` are the definition of correct for every language binding.

## What gets tested

Each value in [`recorder/src/cases.rs`](recorder/src/cases.rs) is sent three ways, because each one reaches a different check in AppSync:

| Path | How the value reaches AppSync |
|---|---|
| `input_variable` | As a GraphQL variable |
| `input_literal` | Written inline in the query |
| `output` | Returned by a resolver for a field typed as the scalar |

The API in [`infra/echo-api.yaml`](infra/echo-api.yaml) uses a `NONE` data source, so it stores nothing and needs no Lambda function. Input fields return `String` and output fields take a `String`, so each request exercises only one check.

## Vector format

`vectors/aws-date.json`, `aws-time.json`, `aws-datetime.json` and `aws-timestamp.json` each hold a list of cases:

```json
{
  "group": "leap-year",
  "value": "2020-02-29",
  "input_variable": { "result": "accepted", "value": "2020-02-29" },
  "input_literal": { "result": "accepted", "value": "2020-02-29" },
  "output": { "result": "accepted", "value": "2020-02-29" }
}
```

`result` is one of:

- `accepted`: `value` is what the resolver saw (input paths) or what the client received (output). A difference from the original means AppSync changed the value.
- `rejected`: `error` holds AppSync's message.
- `syntax_error`: the query could not be parsed, so the scalar check never ran.

`vectors/scalar-descriptions.json` holds AppSync's own description of each scalar, taken from introspection.

Output resolvers use VTL and return the JSON text unchanged, because AppSync rejected a valid `AWSTimestamp` that a JavaScript resolver returned as a bare number. Other JavaScript resolver shapes, such as an object with a timestamp field, are untested. Input resolvers use JavaScript, so a number beyond 2^53 is rounded in the recorded `value` after AppSync has already accepted it.

## How results stay trustworthy

- Control values must pass and fail as expected on every path before recording starts.
- Only recognized scalar errors count as rejections. Any other error stops the run without writing files.
- Throttling and server errors are retried. A request that keeps failing stops the run, so it is never recorded as a rejection.
- Every case runs twice by default, and any disagreement stops the run.
- Files contain the region and date, but no account ID, API URL or key.

## Recording

You need Rust, the AWS CLI and credentials that can manage AppSync and CloudFormation.

```bash
AWS_PROFILE=<profile> conformance/live.sh all
```

This deploys the stack, records, and deletes the stack, even if recording fails. It defaults to `us-east-1`; set `AWS_REGION` to change that. A run sends about 5,000 queries, which costs about $0.02 at AppSync's rate of $4 per million.

To keep the API up while exploring, run the steps separately:

```bash
AWS_PROFILE=<profile> conformance/live.sh deploy
```

```bash
AWS_PROFILE=<profile> conformance/live.sh probe AWSDateTime '"2021-01-11T12:30:24"'
```

```bash
AWS_PROFILE=<profile> conformance/live.sh record
```

```bash
AWS_PROFILE=<profile> conformance/live.sh teardown
```

Run the offline tests with `cargo test`. They need no AWS access.

[`acceptance/`](acceptance/) holds a check for each language that compares an installed package against `smoke.json` and every recorded result the language's API can express. Recorded values of other JSON types, such as `null` or an object sent as an `AWSDate`, are left out, because the functions only take a string or an integer. [`scripts/acceptance.sh`](../scripts/acceptance.sh) runs them.
