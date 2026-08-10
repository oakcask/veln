---
role: specification
authority: normative
update-when: The `veln test --json` output schema, static gate behavior, case records, runtime failure details, expected-output details, stdio event fields, or executable test JSON evidence changes.
---

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
  `source_to_test_convention`, `dependency_graph`, or
  `widened_dependency_graph`
- `notes`: optional human-readable selection notes

With no explicit targets, `veln test --json` discovers selected `*_test.veln`
files and any other discovered source file that contains a top-level `test`
declaration, including multiple test-bearing files in one run. It reports
`confidence: "complete"`. The selected target list is sorted and includes each
selected test-bearing file path once.

With explicit targets, the command treats the caller's direct file or recursive
directory list as intentional selection and reports `reason: "user_selected"`
unless it adds tests by convention or dependency graph. If an explicit non-test
`.veln` file has a same-directory `*_test.veln` peer with the same base name,
the peer is added to the selected targets and the run records a selection note.

For explicit non-test source targets with path-derived module identities,
`test` also uses source-level `use` declarations as a dependency graph. A
discovered test source whose transitive imports include the selected source is
included in the final selection. When the graph selects or confirms a test
source and all needed graph evidence is present, the run reports
`reason: "dependency_graph"` and `confidence: "complete"`. If graph evidence
is missing, such as an import with no discovered source module, the command
widens to all discovered tests, reports `reason: "widened_dependency_graph"`,
and reports `confidence: "unknown"`. The reported `targets` array is the final
selected test roots and is sorted after duplicate removal.

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
`failure.kind: "runtime"`. This includes a missing `java` before compilation
or class loading.

Runtime contract failures inside a selected test case are reported as failed
cases with `failure.kind: "contract"`. The failure details use
`kind: "contract"` and `phase: "runtime"` and include:

- `clause`: `require`, `ensure`, or `invariant`
- `predicate`: the failed clause text
- `function`: the checked function or test boundary
- `blame`: `caller` for `require`, `implementation` for `ensure`, and either
  value for `invariant` depending on entry or return failure
- `node_id`: the contract node identifier
- `span`: the source span for the failed clause

Tests or doctests that return `Err(value)` are reported as failed cases with
`failure.kind: "result"`. The failure details use `kind: "result"` and
`phase: "runtime"` and include `value`, the formatted returned error value.

Doctests with runtime failure metadata pass when the runtime failure details
match the expected contract, ensure, or result failure. The pass case omits a
failure record. If execution succeeds or produces a different runtime failure,
the case is reported with `status: "failed"`,
`reason: "expected_runtime_failure"`, and
`failure.kind: "runtime_expectation"`. The failure details use
`kind: "runtime_expectation"` and include:

- `expected`: the expected runtime failure metadata. Contract expectations
  include `kind`, `clause`, `predicate`, the metadata fence `span`, and any
  supplied `function` or `blame`. Ensure expectations include `kind`,
  `predicate`, the metadata fence `span`, and any supplied `function` or
  `blame`. Result expectations include `kind`, `value`, and the metadata fence
  `span`.
- `actual`: the actual runtime failure record, or `null` when execution
  succeeded

Static diagnostics still block execution before runtime expectation matching;
the doctest case is then reported with `status: "blocked"` and
`reason: "static_gate"`. The implemented runtime expectation kinds are limited
to contract, ensure, and result failures; there is no test JSON expectation
record for arbitrary panics, raw stderr matching, or process exit status.

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
`Result<_, E>`. The generated doctest wrapper returns `Result<(), E>` and
appends the implicit success value before static gates run.

Doctest fences marked `veln ignore` are documentation-only examples. They do
not produce generated test sources, case records, expected-output attachments,
or static diagnostics from their body.

Doctest fences marked `veln fail` are negative static examples. Diagnostics
from the generated negative source satisfy the expectation and are removed from
the top-level diagnostics only when at least one matching diagnostic has
severity `error`. Hint-only diagnostics remain in the top-level diagnostics and
do not satisfy the expectation. If the generated source produces no error
diagnostic, the run reports `doctest.expected_failure_missing` as a static doc
diagnostic. Negative doctests do not produce case records or expected-output
attachments.

Executable doctest lines that start with `> ` are hidden setup lines. The
generated doctest includes each hidden setup line after removing the marker,
and diagnostics for that generated source use the normal doctest source path.
Lines that start with `#` remain visible source comments inside the generated
doctest source.

Unknown doctest metadata and invalid doctest metadata are also reported as
static doc diagnostics before execution. Unknown `veln` and `veln-output`
attributes use `doctest.unknown_metadata`; empty `error=`, empty or unsupported
runtime expectation metadata, missing runtime expectation details, missing
`stream`, duplicate `stream` attributes on one output fence, and unsupported
output streams use `doctest.invalid_metadata`.

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
When runtime tracing is available, stdio operations are serialized before
capture. The `sequence` field defines the observed operation order across
stdout and stderr, including output produced by spawned tasks.

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
