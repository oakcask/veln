# Commands

This file specifies implemented CLI behavior for the first slice.

## `veln check [--json] [path ...]`

`check` discovers source files, parses them, lowers each parse-clean file to the
surface AST, runs semantic diagnostics for that file, and then lowers each
error-free file far enough to report checked-core executable blockers such as
call and constructor arity mismatches. With `--json`, it prints the check JSON
envelope. Without `--json`, it prints human diagnostics or `ok`.

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
diagnostics.

This command currently analyzes files independently. Cross-file semantic
analysis is an implementation gap, not an implied part of the fixed check
behavior.

## `veln fmt [path ...]`

`fmt` uses the same source discovery rule as `check`. It parses every selected
file before writing any file. If any parse diagnostic is present, the whole
format invocation exits with failure and writes nothing.

For parse-clean files, formatting is deterministic for the implemented syntax:
module and use headers, function signatures, contract clauses, let statements,
tail expressions, holes with `satisfy`, records, lists, calls, literals, paths,
prefix operators, binary operators, and postfix `?`.

Files containing comments are preserved byte-for-byte until formatter comment
attachment is implemented.

## `veln run <entry> [path ...] [-- arg ...]`

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
core, then typed IR, then generated Java source. Semantic diagnostics in
functions unreachable from the selected entry do not block `run`. The command
writes generated Java artifacts to an isolated temporary build directory,
invokes `javac`, invokes `java`, forwards process stdout and stderr, and
returns the Java process status for runtime failures.

Missing `javac` before compilation or missing `java` after compilation
succeeds is reported as a JDK setup error.

## `veln test [--json] [target ...]`

`test` reuses the parser, semantic diagnostics, checked-core lowering, typed IR,
JVM backend, and Java execution path used by `run`.

Like `run`, `test` combines parse-clean selected files into one surface module
before semantic analysis.

Without explicit targets, `test` selects top-level `test` declarations in
discovered `*_test.veln` files. With explicit targets, it selects `test`
declarations from the selected files, including files found recursively below
explicit directories and including non-test files. Ordinary `fn` declarations
are never selected merely because they have zero parameters.

When an explicit target names a non-test `.veln` source file, `test` also
selects a same-directory `*_test.veln` file with the same base name when that
paired file exists. The command reports this as `source_to_test_convention` in
JSON output and prints a human selection note. The selection confidence is
`partial` because the convention is narrower than a complete dependency graph.

Static diagnostics block the suite before Java execution. In JSON output,
already discovered cases are marked `blocked` with reason `static_gate`.
Runtime failures become failed cases. JDK setup failures become case errors with
reason `runner_error`, including a missing `java` after `javac` succeeds.
