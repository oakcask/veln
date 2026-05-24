# Test JSON

This file specifies the implemented JSON output for `veln test --json`.

## Envelope

`veln test --json` emits schema version `veln-test-json/v0` with:

- `command`
- `status`: `passed`, `failed`, `blocked`, or `error`
- `selection`
- `summary`
- `diagnostics`
- `suite_errors`
- `cases`

## Selection

Selection fields are:

- `mode`: `discovered` or `explicit`
- `targets`
- `confidence`: `complete`, `partial`, or `unknown`
- `reason`: `pattern_discovery`, `user_selected`, or
  `source_to_test_convention`
- `notes`: optional human-readable selection notes

With no explicit targets, `veln test --json` discovers all selected
`*_test.veln` files and reports `confidence: "complete"`.

With explicit targets, the command treats the caller's direct file or recursive
directory list as intentional selection and reports `reason: "user_selected"`
unless it adds paired tests by convention. If an explicit non-test `.veln` file has a
same-directory `*_test.veln` peer with the same base name, the peer is added to
the selected targets, the run reports `reason: "source_to_test_convention"`,
and `confidence: "partial"` because the mapping is conservative but not a full
dependency graph.

## Summary

Summary fields are:

- `total`
- `passed`
- `failed`
- `skipped`
- `todo`
- `blocked`
- `errors`

## Cases

Each case has:

- `id`
- `name`
- `kind`
- `status`
- `source`
- `reason`
- `failure`
- `events`
- `diagnostics`

Source `test` declarations use `case.kind: "test"` and a `source.node_id`
prefix of `test`. Ordinary functions use the `fn` prefix in other diagnostic
contexts but are not selected as test cases.

JDK setup failures are reported on the affected case with
`status: "error"`, `reason: "runner_error"`, and
`failure.kind: "runtime"`. This includes a missing `javac` before compilation
and a missing `java` after compilation succeeds.

Captured stdio events use:

- `kind: "stdio"`
- `stream`: `stdout` or `stderr`
- `operation`: `print`, `println`, `eprint`, or `eprintln`
- `text`: the string passed to the stdio operation, without the logical
  newline for `println` or `eprintln`
- `terminator`: `none` for `print` and `eprint`, or `newline` for `println`
  and `eprintln`
- `sequence`: a monotonic integer within the case
- `node_id`: the source call node identifier
- `span`: the source call span

The event list is operation-oriented. `println` and `eprintln` preserve their
logical newline through `terminator`, not by appending it to `text`. If runtime
tracing is unavailable, output may be represented as aggregate stdout or stderr
events attached to the case source.
