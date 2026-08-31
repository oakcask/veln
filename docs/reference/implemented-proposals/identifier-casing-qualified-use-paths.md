---
role: implementation-record
authority: supporting
update-when: Qualified-use path identifier casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Qualified Use Paths

## Outcome

Qualified expression, pattern, and type paths now use the source identifier
casing diagnostic contract for every written segment whose role is fixed by
syntax, successful resolution, or one unique recovery link. Current behavior
is specified by
[Name Resolution](../../specification/name-resolution.md),
[Check JSON And Diagnostics](../../specification/diagnostics-json.md), and
[Editor Support](../../specification/editor-support.md). The checked
`identifier-casing-qualified-use-paths-json` and
`identifier-casing-qualified-use-paths-human` examples fix the JSON and human
command behavior for the main path matrix. The checked
`identifier-casing-qualified-use-recovery-controls-json` and
`identifier-casing-qualified-use-recovery-controls-human` examples fix the
same-source recovery and unresolved-control boundaries. The checked
`identifier-casing-qualified-use-navigation`,
`identifier-casing-qualified-module-type-navigation`,
`identifier-casing-qualified-prelude-navigation`, and
`identifier-casing-qualified-function-navigation` LSP examples fix the
definition, references, prepare-rename, rename, immutable package symbol, and
unsupported segment-selection boundaries for qualified path segments.

## Scope

The implemented slice retains one token span for each written expression and
type path segment through parsing, AST lowering, and AST wire round trips.
Module-only function and value paths validate resolved or recovered qualifier
segments as `module` and validate the final segment as `function` for calls or
`value_binding` for value references. Module-and-type constructor paths
validate resolved or recovered module qualifiers as `module`, the type
qualifier as `type`, and the final segment as `constructor`.
Prelude-qualified function calls use the module and function classes.
Prelude-qualified type and constructor paths use the same module, type, and
constructor classes. Unresolved or ambiguous intermediate segments are not
classified from spelling alone.

Each invalid role-fixed segment emits `name.invalid_case` with `phase: name`,
`origin: source`, `occurrence: path_segment`, the exact written segment
spelling, the role-fixed `name_class`, the required and observed initial
classes, and the zero-based `segment_index`. A call-target diagnostic whose
only cause is the resolved or uniquely recovered invalid segment that owns the
use is suppressed. Missing targets, private imported targets, and recovery
links that would cross an import boundary remain ordinary unresolved
failures.

The language service selects supported type, constructor, function, and
package prelude symbols through the retained segment ranges. Module-only
qualified public functions, constructor-qualified type segments, imported
module-and-type constructor paths, and their constructor leaves support
definition, references,
prepare-rename, rename, and class-changing rename rejection according to the
current rename contract. Standard-library package symbols return package
definition locations, no workspace references, no prepare-rename range, and
empty rename edits. Module segments and other unsupported segment roles have
no selected symbol.

## Completion

This slice is complete for role-fixed qualified-use path casing diagnostics
and the covered qualified-use language-service operations. It does not
complete module identity syntax, explicit import-alias syntax, recovery
navigation through quarantined invalid declarations, repair rename, rename
conflict prediction, MCP rename mapping, or source migration beyond the
focused executable examples.
