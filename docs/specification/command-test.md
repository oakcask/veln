---
role: specification
authority: normative
update-when: The veln test command selection, parallel execution, source-error gate, runtime failure, or output behavior changes.
---

# Test Command

`test` reuses the parser, semantic diagnostics, checked-core lowering, typed IR,
JVM backend, and Java execution path used by `run`, including the generated JVM
class cache.

Like `run`, `test` combines parse-clean selected files into one surface module
before semantic analysis.
Source identifier casing diagnostics inside the selected test analysis set
keep the selected-suite static gate, mark selected cases as blocked, and
prevent JVM artifact generation. Source identifier casing diagnostics outside
the selected test analysis set are not reported by that invocation and do not
block the selected suite. The
`identifier-casing-source-path-artifact-gate-json` and
`identifier-casing-source-path-unselected-artifact-json` executable examples
check this boundary for source-path-derived module identity casing.

`-j <JOBS>` and `--jobs <JOBS>` set the maximum number of runnable test cases
that may execute concurrently. `JOBS` is a positive decimal integer. When the
option is omitted, `test` uses the process's available parallelism and falls
back to one job if availability cannot be determined. The active worker count
is clamped to the runnable case count, and an empty runnable set starts no
workers. `--jobs 1` is the serial compatibility route for suites whose cases
share external state. The job option is recognized before `--`, including
after a target; after `--`, the same token is a target. Zero, missing,
malformed, repeated, mixed-spelling repeated, and overflowing job values are
command-line errors before project discovery.

Without explicit targets, `test` selects top-level `test` declarations in
discovered `*_test.veln` files and in any other discovered source file that
contains a top-level `test` declaration. With explicit targets, it selects
`test` declarations from the selected files, including files found recursively
below explicit directories and including non-test files. Ordinary `fn`
declarations are never selected merely because they have zero parameters.

`test` also extracts executable doctests from documentation line comments.
A doctest starts with a doc comment fence whose info string is `veln` and is
checked as a generated private `test` declaration. By default the generated
test returns `()` and declares `effects [stdio]`. A doctest fence may include
an `error=<TypePath>` info-string attribute. With that attribute, the
generated test returns `Result<(), <TypePath>>` and appends `Ok(())` as the
implicit success value, so the visible example body can use `?` without
writing harness-only success code. Without `error=<TypePath>`, a doctest that
uses `?` can infer the wrapper error type from the immediately documented
public `Result<_, E>` function or from known propagated function calls when all
of them use the same `E`. A `veln ignore` fence is documentation-only: it is
not generated, checked, selected, or paired with expected output. A `veln fail`
fence is a negative static example: `check` and `test` accept it only when its
generated source produces at least one error diagnostic; hint-only diagnostics
do not satisfy the expected failure. A negative doctest is not selected as a
runtime doctest case. A positive doctest fence may include
`runtime=contract clause=<Clause> predicate=<Predicate>` to expect a runtime
contract failure after static checking succeeds. The contract expectation
matches `require`, `ensure`, or `invariant` by contract failure kind, runtime
phase, clause, and predicate; optional `function=<Name>` and `blame=<Side>`
attributes further constrain the match. A positive doctest may instead include
`runtime=ensure predicate=<Predicate>` to expect a runtime `ensure` contract
failure after static checking succeeds, with optional `function=<Name>` and
`blame=<Side>` constraints. A positive doctest may also include
`runtime=result value=<FormattedValue>` to expect the generated test to return
`Err(<FormattedValue>)`. Other unknown executable doctest attributes, empty
metadata values, missing runtime contract `clause` or `predicate`, missing
runtime ensure `predicate`, missing runtime result `value`, and unsupported
runtime expectation kinds are static doc diagnostics. A line inside an
executable doctest fence that starts with `> ` is hidden setup: the generated
test includes the line after the marker, so the example can bind helpers
without exposing harness code in the documented sample. `# comment` lines stay
visible source comments inside generated doctests. The hidden marker is exact
after the doc-comment prefix and one optional separator space; an example that
intentionally starts source with `>` can write one extra leading space before
`>`. In `check`, generated doctests participate in parse and semantic
diagnostics. In `test`, generated positive doctests are selected as doctest
cases.

An adjacent doc comment fence whose info string is
`veln-output stream=stdout` or `veln-output stream=stderr` records expected
output for the immediately preceding executable doctest. Unknown output-fence
attributes, missing `stream`, duplicate `stream` attributes, and unsupported
stream values are static doc diagnostics. When at least one output fence is
present, any stream without a fence is expected to be empty. Output comparison
uses captured stdio events, reconstructs logical stdout and stderr text, and
ignores the Markdown closing-fence newline as a raw byte assertion.

When an explicit target names a non-test `.veln` source file, `test` also
selects a same-directory `*_test.veln` file with the same base name when that
paired file exists. The command records this in JSON output and prints a human
selection note.

For explicit non-test source targets with path-derived module identities,
`test` builds a source-level dependency graph from `use` declarations. Tests whose
transitive imports include the selected source are included in the selected
test roots before semantic analysis. If the graph is incomplete, for example
because an import has no discovered source module, `test` reports the missing
evidence and widens to all discovered tests instead of silently
under-selecting. Selected cases, static diagnostics, and JSON selection
metadata all observe the final selected target set.

Static diagnostics block the suite before Java execution. In JSON output,
already discovered cases are marked `blocked` with reason `static_gate`.
Runtime failures become failed cases. Runtime contract failures inside a
selected case use `failure.kind: "contract"` and include runtime contract
details. Tests or doctests that return `Err(value)` use
`failure.kind: "result"` and include the formatted error value. A doctest with
runtime failure metadata passes that route only when the actual runtime failure
matches the expected details. If execution succeeds or fails differently, the
case fails with
`failure.kind: "runtime_expectation"` and
`reason: "expected_runtime_failure"`. If static diagnostics block execution,
the discovered doctest case is blocked with `reason: "static_gate"`. JDK setup
failures become case errors with reason `runner_error`, including a missing
`java` before class loading.

When the static gate passes, every runnable case is scheduled through the
bounded ordered executor. The coordinator constructs and renders the report
only after all workers finish, so human status lines, JSON `cases`, diagnostics,
captured events, summary counts, failures, and exit status remain in discovered
case order regardless of completion order. Worker stdout and stderr are
captured per case and are not streamed while workers are active. The checked
examples under `../../examples/specification/test/parallel-jobs-one-json/`,
`../../examples/specification/test/parallel-jobs-two-json/`, and
`../../examples/specification/test/parallel-jobs-auto-json/` are the primary
observable specification for ordered JSON case records across serial, bounded,
and automatic job modes.

Runtime failure expectation matching is independent from expected-output
comparison. Satisfying `runtime=contract`, `runtime=ensure`, or
`runtime=result` does not satisfy any attached output fence, and matching
output does not satisfy the runtime failure expectation. The implemented
runtime expectation surface is limited to those structured contract, ensure,
and result failure kinds. Doctests do not match arbitrary panics, raw stderr
text, or process exit status as runtime failure expectations.

Doctest output mismatches become failed cases with `failure.kind: "output"` and
`reason: "expected_output"`. JSON details include the mismatched stream,
expected text, actual text, first differing logical line, bounded captured
stdio events for the actual stream, and the expected-output fence span when
available.
