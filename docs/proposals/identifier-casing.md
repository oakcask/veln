---
role: proposal
update-when: Remaining identifier casing classes, qualified-name roles, language-service behavior, or acceptance evidence changes.
---

# Complete Identifier Casing

## Outcome

Extend the implemented source declaration and binding casing foundation to
module identities, qualified occurrences, alias target leaves, source-less
symbol registries, and language-service operations.

Current type, constructor, function, public type and function alias declaration
name, and value-binding behavior is specified in
[`names-effects-full.md`](../specification/names-effects-full.md#name-resolution).
Its completed recovery foundation is recorded in
[`identifier-casing-source-recovery.md`](../reference/implemented-proposals/identifier-casing-source-recovery.md).

## Remaining Naming Contract

`Uppercase` means an ASCII letter in `A` through `Z`. `Lowercase` means an ASCII
letter in `a` through `z`.

| Name class | Required initial | Remaining covered source |
| --- | --- | --- |
| Type | Uppercase | Qualified type uses and type-alias target leaves. |
| Constructor | Uppercase | Qualified constructor calls and pattern heads. |
| Module | Lowercase | Written module identities, source-path-derived module segments, import paths, implicit import aliases derived from the final path segment, and future explicit import aliases. |
| Function | Lowercase | Qualified function uses and function-alias target leaves. |
| Value binding | Lowercase | Remaining language-service classified binding occurrences. |

A wrong-cased source occurrence reports `name.invalid_case` at its exact token.
Source-path-derived module segments use a zero-width span at the start of the
source and identify the path segment, source kind, source path, and segment
index in structured details. Casing is checked on user-controlled origin
segments before companion, doctest, generated-source, or export identities add
synthetic text. Synthetic segments are not casing occurrences. A standalone
`_` keeps its implemented wildcard, discard, or structural-error behavior.

## Qualified Resolution

Every resolved qualified occurrence retains a semantic role for each segment.
The shared resolver classifies the longest module or import-alias prefix, an
optional type qualifier, and the final member required by the source position.
Only a role fixed by syntax, successful resolution, or one unique recovery link
receives a casing diagnostic. An unresolved or ambiguous intermediate segment
is not classified from spelling alone.

Public function and type alias kinds fix the class of the alias target leaf.
Alias declaration-name casing is already implemented. Schema alias target
leaves remain casing-neutral. Missing targets and wrong-kind targets keep
their existing diagnostics when those facts are independently provable. An
invalid recovery target does not enter the export namespace.

Qualified patterns use constructor classification for the final segment.
Lowercase qualified pattern heads report constructor casing instead of being
reinterpreted as value bindings. Unresolved and ambiguous intermediate
segments are not classified from spelling alone.

## Command And Language-Service Boundary

Add selected and unselected evidence for `test`, included and excluded source
evidence for `doc`, snapshot and overlay evidence for language services, and
loaded and unloaded dependency evidence. These consumers preserve their
existing selection boundaries; identifier casing must not become a
workspace-global gate.

Definition, references, prepare-rename, and rename use the same classified name
roles as checking. A unique recovery link may support navigation. Rename may
use it only to produce a class-correct repair. Invalid symbols remain absent
from package snapshots and public indexes.

Prepare-rename returns no range for source-path-derived module segments.
Rename rejects class-changing requests with `rename.invalid_case`, rejects
atomic conflicts with `rename.conflict`, and returns no partial edits. LSP maps
these domain failures to JSON-RPC invalid params. A future MCP rename operation
preserves the same shared code and details.

## Source-Less Symbols

Validate source-less lookup registry names before publishing the registry.
The release-mode gate is atomic: either the complete validated registry is
available, or no partial registry is available to lookup. Invalid registry
entries stop command and adapter initialization with the existing internal
failure form.

The diagnostic code is `toolchain.invalid_symbol_case`. It has no source span.
Its details identify at least `provider`, `name`, `name_class`, and
`required_initial`. The gate does not convert this failure into source
`name.invalid_case`, silently skip the entry, panic only in debug builds, or
wait until lookup to validate it. Generated tables use the same validator at
their generation or build gate without removing the release-time check.

Embedded Veln prelude sources remain source-written sources and use
`name.invalid_case`. Compiler temporaries and bookkeeping names that cannot
participate in source lookup are outside this registry contract.

## Diagnostic Overlap And Ordering

Independently provable diagnostics still accumulate. Structural parse or
module diagnostics precede `name.invalid_case` at the same span. Casing
diagnostics precede duplicate, ambiguity, kind, unresolved, type, and lowering
diagnostics at the same span.

Same-kind duplicate declarations with the same invalid spelling still report
duplicates. Alias target kind and missing-target diagnostics remain when the
failure does not depend on treating an invalid declaration as a normal symbol.
Resolution, ambiguity, kind, unresolved, callability, type-origin,
constructor-arity, and exhaustiveness diagnostics are suppressed only when the
failure exists solely because of one unique compatible recovery symbol.

## Migration Audit

Implementation must audit repository-owned source carriers, not only `.veln`
files. The inventory includes embedded standard-library sources, Rust test
strings, generated test sources, executable examples, checked Markdown
doctests, editor and agent service source cases, snapshots, and expected
diagnostics or navigation edits. Only dedicated identifier-casing rejection
fixtures may retain invalid casing, and those fixtures must assert the exact
expected diagnostic id, count, message, and span.

## Acceptance Evidence

- Module cases cover written paths, derived paths, imports, aliases,
  companions, doctests, and generated-source origin metadata.
- Qualified cases cover module, type, constructor, function, and alias-target
  segment roles, including unresolved and ambiguous prefixes.
- Command cases cover `test`, `doc`, language-service, and dependency selection
  boundaries.
- Navigation cases prove shared checking and language-service classification,
  unique recovery links, ambiguous recovery, and class-correct rename edits.
- Registry cases prove rejection before lookup and artifact publication.
- Overlap cases prove structural, reserved-name, duplicate, ambiguity,
  target-kind, unresolved, and recovery-derived cascade behavior in human and
  JSON output.
- Migration audit cases prove all repository source carriers follow the
  contract except dedicated exact-expectation rejection fixtures.

## Non-Goals

- Unicode identifier classes.
- Case-insensitive lookup or automatic case conversion during checking.
- Changing schema, effect, handler, effect-operation, record-field, type
  parameter, or hole-label casing.
