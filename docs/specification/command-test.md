---
role: specification
authority: normative
update-when: The veln test command selection, parallel execution, source-error gate, runtime failure, or output behavior changes.
specification-coverage: usage=#test-command; behavior=#selection-and-jobs; limits=#limits
---

# Test Command

Use `veln test [TARGET ...]` to execute selected top-level `test`
declarations and executable doctests. It shares parsing, semantic analysis,
checked-core lowering, typed IR, JVM execution, and cache behavior with
`run`. Ordinary functions are not selected merely because they have no
parameters.

## Selection and jobs

Without targets, `test` selects top-level tests in discovered
`*_test.veln` files and in any other discovered source containing a top-level
test. Explicit files and recursive directories select tests from those files,
including non-test source files.

An explicit non-test source adds its same-directory `*_test.veln` peer when
the base names match. For explicit source targets, `use` declarations form a
dependency graph: tests that transitively import a selected source are added.
If graph evidence is incomplete, the command widens to all discovered tests and
reports unknown selection confidence in JSON rather than under-selecting.

`-j JOBS` and `--jobs JOBS` set the maximum concurrent runnable cases.
`JOBS` is a positive decimal integer. Without it, the command uses available
process parallelism and falls back to one job when unavailable. The active
worker count is clamped to runnable cases; an empty runnable set starts no
workers. The option is parsed before `--`, including after a target; after
`--`, the token is a test target. Zero, missing, malformed, repeated,
mixed-spelling, and overflowing values fail before discovery.

## Executable doctests

A doc-comment fence with info string `veln` becomes a generated private test.
The default wrapper returns `()` and declares `effects [stdio]`. The
`error=TYPE` attribute changes the wrapper to `Result<(), TYPE>` and adds
implicit `Ok(())`, allowing `?` in the visible body. Without `error`,
`?` can infer the error type from the immediately documented public
`Result<_, E>` function or consistently propagated calls.

The metadata surface is:

| Metadata | Behavior |
| --- | --- |
| `veln ignore` | Documentation-only; no generated source, diagnostics, case, or output expectation. |
| `veln fail` | Negative static example; requires at least one error diagnostic. Hint-only diagnostics do not satisfy it, and it creates no runtime case. |
| `runtime=contract clause=C predicate=P` | Expects a runtime `require`, `ensure`, or `invariant` failure matching kind, phase, clause, and predicate. |
| `runtime=ensure predicate=P` | Expects an `ensure` failure; optional function and blame constraints may be supplied. |
| `runtime=result value=V` | Expects the wrapper to return `Err(V)`. |
| `error=TYPE` | Declares the generated wrapper error type. |
| `veln-output stream=stdout` | Attaches expected stdout to the immediately preceding executable doctest. |
| `veln-output stream=stderr` | Attaches expected stderr to the immediately preceding executable doctest. |

Contract expectations accept optional `function=NAME` and `blame=SIDE`
constraints. Unknown attributes, empty values, missing required metadata,
duplicate output stream attributes, unsupported runtime kinds, missing output
streams, and unsupported stream names are static documentation diagnostics.

Lines beginning exactly with `> ` are hidden setup: the marker is removed in
generated source. `#` lines remain visible comments. A leading extra space
can make a literal source line beginning with `>` visible. Hidden setup is
included in static analysis but omitted from rendered documentation examples.

## Expected output

An adjacent `veln-output` fence applies to the immediately preceding
executable doctest. When any output fence exists, an unfenced stream is expected
to be empty. Comparison reconstructs logical stdout and stderr from captured
stdio events and ignores the Markdown closing-fence newline as a raw-byte
assertion. Runtime expectation matching and output comparison are independent.

## Gates and failures

Static diagnostics block the suite before Java execution. Selected cases become
blocked with reason `static_gate` in JSON. Runtime contract failures use
contract failure details; returned `Err(value)` uses result failure details.
A runtime expectation passes only when the actual structured failure matches.
A different failure or successful execution fails with
`reason: "expected_runtime_failure"`. Missing Java is a case error with
`reason: "runner_error"`.

Workers capture output per case. The coordinator waits for all workers and
renders human status, JSON cases, diagnostics, events, summaries, failures, and
exit status in discovered-case order regardless of completion order.

## Limits

Runtime expectations cover only contract, ensure, and result failures. They do
not match arbitrary panics, raw stderr, or process exit status. Output matching
does not satisfy a runtime expectation, and satisfying an expectation does not
satisfy an output fence.

## References

Implementation: `crates/veln-cli/src/commands/test.rs` and its scheduler.
The machine-facing case fields are specified in [test-json.md](test-json.md).
