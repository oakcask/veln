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

With no explicit targets, `veln test --json` discovers selected `*_test.veln`
files and any other discovered source file that contains a top-level `test`
declaration, including multiple test-bearing files in one run. It reports
`confidence: "complete"`. The selected target list is sorted and includes each
selected test-bearing file path once.

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

Executable doctests extracted from documentation comments use
`case.kind: "doctest"`. Their generated test names are `doctest_N` within one
command result. Their source file in diagnostics is a generated
`#doctest-N_test.veln` path derived from the documented source path.

JDK setup failures are reported on the affected case with
`status: "error"`, `reason: "runner_error"`, and
`failure.kind: "runtime"`. This includes a missing `javac` before compilation
and a missing `java` after compilation succeeds.

Runtime contract failures inside a selected test case are reported as failed
cases with `failure.kind: "contract"`. The failure details use
`kind: "contract"` and `phase: "runtime"` and include:

- `clause`: `require` or `ensure`
- `predicate`: the failed clause text
- `function`: the checked function or test boundary
- `blame`: `caller` for `require`, or `implementation` for `ensure`
- `node_id`: the contract node identifier
- `span`: the source span for the failed clause

Doctest expected-output mismatches are reported as failed cases with
`reason: "expected_output"` and `failure.kind: "output"`. The failure details
use `kind: "output"` and include:

- `stream`: `stdout` or `stderr`
- `expected`: reconstructed expected stream text from the adjacent
  `veln-output` fence
- `actual`: reconstructed actual stream text from captured stdio events
- `first_difference`: the first mismatching logical line, with one-based
  `line`, `expected`, and `actual` fields
- `actual_events`: up to four captured stdio events for the mismatched stream
- `expected_span`: the expected-output fence span when available

Duplicate `veln-output` fences for the same doctest stream are reported as
static doc diagnostics before execution. The diagnostic id is
`doctest.duplicate_output`; its details include `kind: "doctest_metadata"` and
the duplicate `stream`.

Executable doctests that contain `?` may omit `error=<TypePath>` when the
doctest immediately documents a public function whose declared return type is
`Result(_, E)`. The generated doctest wrapper returns `Result((), E)` and
appends the implicit success value before static gates run.

Doctest fences marked `veln ignore` are documentation-only examples. They do
not produce generated test sources, case records, expected-output attachments,
or static diagnostics from their body.

Executable doctest lines that start with `# ` are hidden setup lines. The
generated doctest includes each hidden setup line after removing the marker,
and diagnostics for that generated source use the normal doctest source path.

Unknown doctest metadata and invalid doctest metadata are also reported as
static doc diagnostics before execution. Unknown `veln` and `veln-output`
attributes use `doctest.unknown_metadata`; empty `error=`, missing `stream`,
and unsupported output streams use `doctest.invalid_metadata`.

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

## Static Gate

Parse and semantic diagnostics block the suite before Java compilation or
execution. The top-level status is `blocked`, and diagnostics are reported in
the run-level `diagnostics` array.

If a parse error prevents a test declaration from being parsed, no case is
invented for that broken declaration. Parse-clean selected cases from other
files may still be discovered before the static gate blocks execution.

When semantic diagnostics block execution, every discovered selected case is
reported with `status: "blocked"` and `reason: "static_gate"`, including cases
from other selected test files.
