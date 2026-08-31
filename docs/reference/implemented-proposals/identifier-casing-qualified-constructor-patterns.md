---
role: implementation-record
authority: supporting
update-when: Qualified constructor-pattern identifier casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Qualified Constructor Patterns

## Outcome

Qualified constructor patterns whose final written segment starts with an
ASCII lowercase letter now report the source `name.invalid_case` diagnostic at
that final segment. The checked
`identifier-casing-qualified-constructor-pattern-json` and
`identifier-casing-qualified-constructor-pattern-human` examples fix the JSON
and human command behavior. The checked
`identifier-casing-qualified-constructor-pattern-over-suppression-json`
example fixes the boundary for exhaustiveness recovery coverage. The checked
`identifier-casing-qualified-constructor-pattern-direct-diagnostics-json`
example fixes the boundary between suppressed head-derived cascades and direct
nested-pattern or arm-body diagnostics. The checked
`identifier-casing-qualified-constructor-pattern-type-mismatch-json` example
fixes the boundary for independently provable constructor-pattern type
mismatches.

Current behavior is specified by
[Name Resolution](../../specification/name-resolution.md),
[Types](../../specification/types.md), and
[Diagnostics JSON](../../specification/diagnostics-json.md).

## Scope

The parser keeps a qualified lowercase pattern as constructor-pattern syntax.
Lowering retains the source span for each written path segment. Analysis
validates only the final segment for this completed slice and emits
`occurrence: path_segment`, `name_class: constructor`, and the zero-based
`segment_index` for the final written path segment.

The invalid head is kept only as a recovery constructor pattern. It suppresses
constructor-resolution, constructor-pattern type mismatch, and match
exhaustiveness diagnostics that would exist only because the invalid
constructor head was rejected. Recovery coverage is limited to the constructor
resolved after changing only the invalid final segment's first ASCII lowercase
letter to uppercase and preserving the remaining spelling. Nested pattern
bindings and the match-arm body still receive normal checking.

## Completion

This slice is complete. It does not complete module-identity casing, recovery
navigation, repair rename, or MCP rename mapping. LSP rename conflict
rejection for valid selected workspace symbols is completed separately in
[Identifier Casing Rename Conflicts](identifier-casing-rename-conflicts.md).
Qualified-use path casing is completed separately in
[Identifier Casing Qualified Use Paths](identifier-casing-qualified-use-paths.md).
