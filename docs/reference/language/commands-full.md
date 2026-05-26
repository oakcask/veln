# Commands Full

Read [commands.md](commands.md) first unless you need command-specific
behavior, gates, or output boundaries.

## Command Sections

- [`veln check`](#veln-check)
- [`veln fmt`](#veln-fmt)
- [`veln run`](#veln-run)
- [`veln test`](#veln-test)
- [`veln explain`](#veln-explain)

<a id="veln-check"></a>

## `veln check [--json] [path ...]`

`check` discovers source files, parses them, combines parse-clean files into one
surface module, runs semantic diagnostics for that module, and then lowers it
far enough to report checked-core executable blockers such as missing
expressions plus call and constructor arity mismatches. With `--json`, it
prints the check JSON envelope. Without `--json`, it prints human diagnostics
or `ok`.

Inputs are files or directories. If no path is provided, discovery recursively
selects `.veln` files below the current project root, skipping `.git` and
`target`. Explicit directories are searched recursively. The final discovered
file list is sorted and deduplicated.

If the current project root contains `veln.toml`, the command reads the
implemented `[modules]` manifest table after source discovery. Manifest module
entries are validated only for selected source files; they do not add files to
the selected set and do not override source `mod` declarations.

Semantic diagnostics are suppressed for a file that has parse diagnostics.
Other parse-clean files in the same invocation may still produce semantic
diagnostics. Cross-file facts from parse-clean selected files, including
source-level imports and imported qualified calls, participate in the same
semantic analysis used by `run` and `test`.

<a id="veln-fmt"></a>

## `veln fmt [path ...]`

`fmt` uses the same source discovery rule as `check`. It parses every selected
file before writing any file. If any parse diagnostic is present, the whole
format invocation exits with failure and writes nothing.

For parse-clean files, formatting is deterministic for the implemented syntax:
module and use headers, function signatures, contract clauses, let statements,
tail expressions, holes with `satisfy`, records, lists, calls, literals, paths,
prefix operators, binary operators, and postfix `?`.

Formatting accepts multiple parse-clean input files in one invocation and
writes each selected file only after all selected files have parsed without
diagnostics. The implemented golden coverage includes `ensure` clauses, prefix
and binary precedence, postfix `?`, nested records, lists, calls, and
idempotent formatting across multiple input files.

Standalone line comments attach to the next parsed source line during
formatting. The formatter preserves the comment text and emits it with the
same indentation as the formatted module header, import, function signature,
contract clause, body line, or closing `end` line it documents. Trailing line
comments after source code stay on the same formatted source line.

<a id="veln-run"></a>

## `veln run [--json] <entry> [path ...] [-- arg ...]`

`run` uses the same source discovery rule as `check`. Parse-clean files are
combined into one surface module for entry resolution. It blocks before user
code execution on parse errors, a missing entry function, an entry argument
count mismatch, an entry parameter type that cannot be supplied from command
line text, selected-entry semantic errors, reachable holes, or checked-core
blockers.

The entry must be a discovered function. Arguments after `--` are entry
arguments, not source inputs. Entry parameters may be declared as `String`,
`Int`, `Float`, or `Bool`. `String` arguments are passed through unchanged.
`Int` arguments parse as decimal signed integers, `Float` arguments parse as
JVM double-precision decimal text, and `Bool` arguments must be exactly `true`
or `false`. The reachable program is semantically checked, lowered to checked
core, then typed IR, then generated Java source. Reachability follows imported
qualified calls by resolving the alias from selected-file `use` declarations to
the imported source module. Semantic diagnostics in functions unreachable from
the selected entry do not block `run`.

The command caches compiled Java artifacts by generated source content below
the project-local build output area. On a cache miss it invokes `javac`; on a
cache hit it reuses the cached classes and invokes `java` directly. Runtime
trace files for JSON output remain isolated to the individual command
invocation. Human mode forwards process stdout and stderr and returns the Java
process status for runtime failures.

With `--json`, `run` captures process stdout and stderr into the run JSON
record instead of forwarding them separately. Runtime contract failures are
reported as top-level structured runtime errors with contract details.

Missing `javac` before compilation or missing `java` after compilation
succeeds is reported as a JDK setup error.

<a id="veln-test"></a>

## `veln test [--json] [target ...]`

`test` reuses the parser, semantic diagnostics, checked-core lowering, typed IR,
JVM backend, and Java execution path used by `run`, including the generated
Java artifact cache.

Like `run`, `test` combines parse-clean selected files into one surface module
before semantic analysis.

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
generated test returns `Result((), <TypePath>)` and appends `Ok(())` as the
implicit success value, so the visible example body can use `?` without
writing harness-only success code. Without `error=<TypePath>`, a doctest that
uses `?` can infer the wrapper error type from the immediately documented
public `Result(_, E)` function or from known propagated function calls when all
of them use the same `E`. A `veln ignore` fence is documentation-only: it is
not generated, checked, selected, or paired with expected output. A `veln fail`
fence is a negative static example: `check` and `test` accept it only when its
generated source produces at least one parse or semantic diagnostic. A negative
doctest is not selected as a runtime doctest case. Other unknown executable
doctest attributes and empty `error=` values are static doc diagnostics. A line
inside an executable doctest fence that starts with `# ` is hidden setup: the
generated test includes the line after the marker, so the example can bind
helpers without exposing harness code in the documented sample. In `check`,
generated doctests participate in parse and semantic diagnostics. In `test`,
generated positive doctests are selected as doctest cases.

An adjacent doc comment fence whose info string is
`veln-output stream=stdout` or `veln-output stream=stderr` records expected
output for the immediately preceding executable doctest. Unknown output-fence
attributes, missing `stream`, and unsupported stream values are static doc
diagnostics. When at least one output fence is present, any stream without a
fence is expected to be empty. Output comparison uses captured stdio events,
reconstructs logical stdout and stderr text, and ignores the Markdown
closing-fence newline as a raw byte assertion.

When an explicit target names a non-test `.veln` source file, `test` also
selects a same-directory `*_test.veln` file with the same base name when that
paired file exists. The command reports this as `source_to_test_convention` in
JSON output and prints a human selection note. The selection confidence is
`partial` because the convention is narrower than a complete dependency graph.
This pairing is part of test discovery before semantic analysis, so selected
cases, static diagnostics, and JSON selection metadata all observe the expanded
target set.

Static diagnostics block the suite before Java execution. In JSON output,
already discovered cases are marked `blocked` with reason `static_gate`.
Runtime failures become failed cases. Runtime contract failures inside a
selected case use `failure.kind: "contract"` and include runtime contract
details. JDK setup failures become case errors with reason `runner_error`,
including a missing `java` after `javac` succeeds.

Doctest output mismatches become failed cases with `failure.kind: "output"` and
`reason: "expected_output"`. JSON details include the mismatched stream,
expected text, actual text, first differing logical line, bounded captured
stdio events for the actual stream, and the expected-output fence span when
available.

<a id="veln-explain"></a>

## `veln explain [--list] [diagnostic-id]`

`explain` is a read-only diagnostic catalog command. It does not discover,
parse, check, lower, compile, or run source files.

With a known diagnostic ID, it prints the diagnostic title, a short meaning,
and a repair-oriented note. With `--list`, it prints the IDs available in the
implemented catalog. Unknown IDs and an invocation without either `--list` or
a diagnostic ID are command-line errors.

The implemented catalog covers the first diagnostic families used most often
in the typed-hole and predicate repair loop:

- `hole.unfilled`
- `hole.satisfy_type_mismatch`
- `hole.satisfy_candidate_shadow`
- `hole.satisfy_candidate_unused`
- `parse.contract_predicate`
- `parse.satisfy_candidate`
- `parse.satisfy_arrow`
- `parse.satisfy_predicate`
