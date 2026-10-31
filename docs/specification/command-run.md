---
role: specification
authority: normative
update-when: The veln run command selection, analysis or execution gate, entry argument, runtime diagnostic, stdout, or JSON behavior changes.
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
variadic entries require at least their fixed parameter count. An arity,
unsupported parameter type, or conversion failure blocks execution before
reachable lowering and user code. These failures use the human stderr form
even when `--json` is present; they do not produce a run JSON envelope.

## Analysis and execution gates

Parse, selected source-path module identity, entry resolution, argument
conversion, selected semantic diagnostics, reachable holes, checked-core
readiness, and typed-IR readiness all gate backend launch. Reachability follows
qualified calls through selected-file imports. An explicit alias qualifier and
the import's full written path select the same target identity for reachability;
the [name-resolution specification](name-resolution.md#explicit-import-aliases)
defines their lookup boundary, including the importing package and module that
own each binding. Imports in same-spelled modules from different packages do
not cross-quarantine during reachable lowering. A reachable invalid dependency
declaration blocks the run; unreachable declarations in an imported dependency
and diagnostics in an unimported manifest dependency do not. An unreachable
local function does not block the selected entry. Alias-qualified,
full-written-path, and bare calls retain the selected module identity through
JVM execution when distinct imported modules export the same function name.
The selected entry also retains its declaring-module identity when a reachable
imported module exports a function with the same name; the imported function
cannot replace the JVM launch target.

A module immutable value declaration in the selected analysis is semantically
checked before backend launch. Invalid finite-data initializers, annotation
mismatches, public visibility failures, imported lookup failures, and
dependency cycles across package sources report their semantic diagnostics.
Imported lookup retains package-aware declaration identity. Every
declaration also produces `module.value_not_executable`, so even a valid
initializer cannot supply a value to the selected program.

Variant-refinement errors inside that existing analysis boundary block backend
launch, including errors for which recovery retains a type or constructor
identity. Human and JSON output derive the diagnostic code, details, primary
span, and related notes from the same diagnostic. Entry resolution and argument
validation gate reachable lowering, so an invalid invocation reports its entry
or argument failure before a reachable refinement error.

A reachable `begin` expression or `defer` statement proceeds through
checked-core and typed-IR readiness and executes through the JVM backend. Its
cleanup ordering, failure precedence, and cancellation behavior are specified
by the [execution boundary](execution.md#runtime-readiness-and-host-boundaries).

A selected entry cannot require call-site context because no Veln call
expression invokes it. Direct calls within the reachable program do supply
call-site context, including forwarding through call-site-aware wrappers.
Runtime contracts in those functions can read the supplied built-in value.
Their direct calls to other call-site-aware functions forward that value.
Indirect calls through function values supply call-site context when the
runtime value is call-site-aware. Calling a call-site-aware function from a
runtime contract on an ordinary function blocks execution before backend
launch. This boundary is deliberate: an ordinary function's contract does not
construct hidden call-site context. The exact propagation and remaining
limits are specified by
[Call-site Declarations](call-site-declarations.md).

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
[run JSON](run-json.md). Cleanup failures related to an existing failure are
printed after the primary message in cleanup execution order. Runtime trace
files remain isolated to this invocation.

An excluded constructor returned by explicit schema runtime validation reports
`schema.variant_refinement_mismatch` at the exact supplied byte offset. A
negative supplied offset reports `schema.validation_offset_invalid` before
field traversal. Human mode identifies the failed fact and renders the terminal
schema followed by every containing composed-schema or anonymous-record field,
source-ADT constructor and named or zero-based positional payload, decimal list
or vector index, and dictionary key before the rejected field. It renders
decoded dictionary-key text and the mismatch or offset reason as related
context. Direct and mutual recursive source-ADT values retain the constructor
and payload segments for every visited value occurrence, including repeated
uses of the same validation shape.

A dynamic integer zero divisor inside a schema predicate stays inside the
schema-validation result boundary. Explicit validation reports
`schema.validation_failed` with the supplied offset and field or schema path;
a generated binary decoder returns its schema predicate division failure as an
ordinary entry `Err`. Neither path exposes a JVM arithmetic exception. The
predicate and decoder boundaries are specified by the
[execution specification](execution.md#runtime-readiness-and-host-boundaries),
and `--json` projection is specified by [run JSON](run-json.md).

If `net::system()` returns a host socket failure from `listen` or `connect` and
the selected entry returns that `Err`, human mode prints the stable transport
failure message instead of the rendered `NetError` value. The primary message
retains the failed operation and transport category. Related notes report the
known local or peer endpoint, the lifecycle phase, false input, output, and
ownership commit facts, and the platform cause. A plain `Err` without carried
diagnostic provenance keeps the ordinary rendered-result behavior.

A computed zero remainder divisor fails with `runtime.remainder_by_zero` and
the message `remainder divisor is zero` at the divisor location. A related note
identifies the complete remainder operation. Output produced before the
failure is retained, and later statements do not execute. The JSON projection
is specified by [run JSON](run-json.md). When the operation occurs in a
runtime contract predicate, this runtime failure is reported instead of a
contract-violation diagnostic.

## Limits

The command does not execute unreachable code, acquire dependencies, or fall
back to another cache root after a configuration failure. Static gates prevent
artifact generation and Java launch, so no user output is produced before a
static failure. A call-site-aware entry is such a static failure even when its
body does not read the built-in value.

## References

Implementation: `crates/veln-cli/src/commands/run.rs`. JVM report projection:
`crates/veln-cli/src/commands/run_report.rs`. Machine fields and runtime
projections: [run-json.md](run-json.md).
The
[`schema-runtime-recursive-source-adt-validation`](../../examples/specification/run/schema-runtime-recursive-source-adt-validation/)
case checks a deep recursive mismatch path through the JSON command boundary.
The command-boundary examples cover
reachable acceptance under
`examples/specification/run/adt-variant-refinement-command-reachability/`
and recovered annotations under the adjacent
`adt-variant-refinement-command-recovery-human/` and
`adt-variant-refinement-command-recovery/` cases. The
[`explicit-import-alias-entry-target`](../../examples/specification/run/explicit-import-alias-entry-target/)
case checks selected-entry identity when an imported function has the same
name. The
[`explicit-import-alias-owner-package-scope`](../../examples/specification/run/explicit-import-alias-owner-package-scope/)
case checks that application and standard-library import owners with the same
module spelling remain independently executable.
