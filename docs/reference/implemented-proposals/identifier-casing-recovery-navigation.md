---
role: implementation-record
authority: supporting
update-when: Identifier casing recovery-navigation evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Recovery Navigation And Rename

## Outcome

Source-written declarations, function parameters, result bindings, local and
pattern bindings, satisfy candidate bindings, handler context parameters, and
handler operation-clause parameters quarantined by `name.invalid_case` now
support shared language-service recovery navigation when exactly one
compatible recovery record is visible and no valid symbol wins. LSP rename for
the same selected recovery records edits the retained declaration and linked
in-scope references when the replacement keeps the recovery record's semantic
name class and does not create a predictable same-namespace or lexical
conflict. Current behavior is specified by
[Editor Support](../../specification/editor-support.md) and checked by the
`identifier-casing-recovery-navigation` and
`identifier-casing-handler-binding-navigation` LSP examples plus focused
`veln-language-service` navigation and rename-conflict tests. MCP `definition`
exposes the same unique recovery definition boundary as specified by
[MCP Workspace Projects, Diagnostics, And Definitions](../../specification/mcp.md)
and checked by the `definition-recovery-navigation` MCP example plus focused
`veln-mcp` tests.

## Scope

The implemented slice covers definition, references, prepare-rename, and LSP
rename for invalid source declarations and bindings retained in the selected
workspace source. Selection may start at the invalid declaration or at a
linked in-scope use. Definition returns the invalid declaration range.
References return only occurrences that resolve back to the same recovery
identity. Prepare-rename returns the selected identifier range. Rename returns
workspace edits for the invalid declaration and those linked references.
Invalid-case and predictable conflict failures return no workspace edits.

A valid class-compatible symbol takes precedence over recovery. Multiple
compatible recovery records with the same spelling do not select an arbitrary
record. Incompatible occurrence roles, shadowing, qualified occurrences that
do not already resolve through an implemented semantic path, occurrences
before a local binding starts, and occurrences outside the invalid declaration
or binding lexical scope do not link to recovery. Source-declared bare nullary
constructor expression and pattern uses remain valid constructor navigation
targets when an invalid same-spelled binding or function recovery record is
also visible.

The `identifier-casing-recovery-navigation` LSP example checks those
selection and rejection rows through definition, references, prepare-rename,
successful rename edits, invalid-case rename failure, and conflict rename
failure where the operation has an observable unsupported-symbol result,
including a callable parameter call target and valid bare nullary constructor
precedence. The
`identifier-casing-handler-binding-navigation` LSP example checks the same
operation set for invalid handler context and operation-clause bindings.
Focused language-service tests provide binding-form coverage for parameters,
callable parameters, result bindings, local bindings, callable local bindings,
local-binding initializer exclusion, pattern bindings, satisfy candidate
bindings, handler bindings, and declaration-form coverage for invalid
constructor declarations, test declarations, public type aliases, and public
function aliases. They also cover valid nullary constructor precedence over
function or binding recovery.

Recovery records remain quarantined. They do not enter normal workspace or
package symbol indexes, direct-dependency lookup, standard-prelude lookup,
cross-import visibility, exact-companion privilege, lowering, or backend
artifacts. MCP exposes no references, prepare-rename, rename edits, dependency
locations, or standard-library locations for recovery selections.

## Completion

This slice is complete for shared language-service and LSP definition,
references, prepare-rename, and repair rename for source declaration and
binding recovery records. It is complete for MCP `definition` recovery
conversion. It does not complete module identity casing, MCP rename mapping,
or deferred artifact consumers.
