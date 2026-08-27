---
role: implementation-record
authority: supporting
update-when: Public alias target-leaf identifier casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Public Alias Targets

## Outcome

Public function-alias and public type-alias target leaves now use the same
source identifier casing diagnostic contract as other covered source names.
Public schema-alias target leaves remain casing-neutral.

Current behavior is specified by
[Names And Effects](../../specification/names-effects.md). The checked
`identifier-casing-public-alias-targets-json` and
`identifier-casing-public-alias-targets-human` examples fix exact JSON and
human diagnostics.

## Scope

The implemented slice retains the exact target leaf token span through parsing,
AST lowering, and AST wire round trips. A public function-alias target leaf
requires an ASCII lowercase initial. A public type-alias target leaf requires
an ASCII uppercase initial.

The target-leaf diagnostic uses `name.invalid_case` with `phase: name`,
`origin: source`, `occurrence: alias_target`, the exact target spelling, the
fixed `function` or `type` class, and the required and observed initial
classes. The diagnostic is emitted before independently provable target
resolution failures at the same alias declaration. Wrong-kind targets still
report `name.kind_mismatch`. Missing targets still report `name.unresolved`.

A public alias with an invalidly cased function or type target leaf does not
enter the normal function or type export namespace, even when the target
otherwise resolves.

## Completion

This slice is complete. It does not add module-identity casing, non-leaf
qualified-use segment casing, recovery navigation, rename behavior, or any
casing rule for schema-alias targets, schema names, effects, handlers,
operations, fields, type parameters, or holes. Source-less registry validation
is completed separately in
[Identifier Casing Source-Less Symbols](identifier-casing-source-less-symbols.md).
