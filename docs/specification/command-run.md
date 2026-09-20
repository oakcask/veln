---
role: specification
authority: normative
update-when: The veln run command selection, execution gate, entry argument, runtime diagnostic, stdout, or JSON behavior changes.
specification-coverage: usage=#run-command; behavior=#analysis-and-execution-gates; limits=#limits
---

# Run Command

Use `veln run [--json] ENTRY [INPUTS ...] [-- ENTRY_ARGS ...]` to execute one
discovered entry. `ENTRY` is the required function name; `INPUTS` are source
files or directories. `--json` captures the result in the [run JSON](run-json.md) envelope;
without it, human diagnostics and process streams are used.

## Entry and arguments

The entry must be a discovered function. Arguments after `--` are entry
arguments, not source paths. Parameters may be `String`, `Int`, `Float`,
or `Bool`. Strings pass through unchanged; integers parse as decimal signed
integers; floats parse as JVM double-precision decimal text; booleans must be
exactly `true` or `false`.

A final variadic parameter may use those element types and gathers additional
arguments into `List<T>`. Non-variadic entries require exact arity;
variadic entries require at least their fixed parameter count. Conversion or
arity failure blocks execution before user code.

## Analysis and execution gates

Parse, selected source-path module identity, entry resolution, argument
conversion, selected semantic diagnostics, reachable holes, checked-core
readiness, and typed-IR readiness all gate backend launch. Reachability follows
qualified calls through selected-file `use` aliases. A reachable invalid
dependency declaration blocks the run; unreachable declarations in an imported
dependency and diagnostics in an unimported manifest dependency do not. An
unreachable local function does not block the selected entry.

The reachable program is lowered to typed IR and JVM classfiles. Ordinary
execution does not write Java source or invoke a Java source compiler. Missing
Java before class loading is a JDK setup failure.

## JVM cache

Generated classfiles are cached by backend content below the selected user
cache root. The default is selected by operating system:

| System | Default base and root |
| --- | --- |
| Unix other than macOS | `XDG_CACHE_HOME/veln` when `XDG_CACHE_HOME` is absolute and non-empty; otherwise `HOME/.cache/veln` under an absolute, non-empty `HOME`. |
| macOS | `HOME/Library/Caches/veln` under an absolute, non-empty `HOME`. |
| Windows | `LOCALAPPDATA/veln` under an absolute, non-empty `LOCALAPPDATA`. |

`VELN_CACHE_DIR`, when set, must be non-empty and lexically absolute and names
the complete cache root without an added `veln` component. Native strings are
retained without canonicalization.

Java availability is checked before cache configuration. Cache configuration is
checked only after source analysis, entry selection, and JVM generation.
Empty or relative overrides do not fall back to a host base; unavailable or
unusable roots do not fall back to the package, working directory, `target`,
or a temporary directory. Commands that do not reach JVM execution do not
inspect cache configuration.

A cache miss publishes emitted classfiles. A hit validates its manifest and
classfiles before Java launch. Invalid entries are removed and replaced before
execution. If removal fails, the existing invalid entry remains and the
command reports a cache error before Java launch; it does not execute or select
a fallback root. If removal succeeds but preparation, validation, or
publication fails, no partial replacement is published. Concurrent writers
use only complete entries matching their generated program; a losing writer
revalidates the published winner and does not delete it. Abandoned cache
coordination produces a bounded pre-launch error; remnants are never executed.

## Human output and errors

Human mode forwards process stdout and stderr and returns the Java status for
ordinary runtime failures. Contract failures identify the failed clause at its
source span and place blame and related facts in diagnostic details. Decode,
schema, byte-write, and protocol failures use the diagnostic shapes routed by
[run JSON](run-json.md). Runtime trace files remain isolated to this invocation.

## Limits

The command does not execute unreachable code, acquire dependencies, or fall
back to another cache root after a configuration failure. Static gates prevent
artifact generation and Java launch, so no user output is produced before a
static failure.

## References

Implementation: `crates/veln-cli/src/commands/run.rs`. JVM report projection:
`crates/veln-cli/src/commands/run_report.rs`. Machine fields and runtime
projections: [run-json.md](run-json.md).
