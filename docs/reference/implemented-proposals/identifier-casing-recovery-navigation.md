---
role: implementation-record
authority: supporting
update-when: Identifier casing recovery-navigation evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Recovery Navigation

## Outcome

Source-written declarations, function parameters, result bindings, local and
pattern bindings, satisfy candidate bindings, handler context parameters, and
handler operation-clause parameters quarantined by `name.invalid_case` now
support shared language-service recovery navigation when exactly one
compatible recovery record is visible and no valid symbol wins. Current
behavior is specified by [Editor Support](../../specification/editor-support.md)
and checked by the `identifier-casing-recovery-navigation` and
`identifier-casing-handler-binding-navigation` LSP examples plus focused
`veln-language-service` navigation tests.

## Scope

The implemented slice covers definition, references, and prepare-rename for
invalid source declarations and bindings retained in the selected workspace
source. Selection may start at the invalid declaration or at a linked
in-scope use. Definition returns the invalid declaration range. References
return only occurrences that resolve back to the same recovery identity.
Prepare-rename returns the selected identifier range.

A valid class-compatible symbol takes precedence over recovery. Multiple
compatible recovery records with the same spelling do not select an arbitrary
record. Incompatible occurrence roles, shadowing, qualified occurrences that
do not already resolve through an implemented semantic path, and occurrences
outside the invalid declaration or binding lexical scope do not link to
recovery.

The `identifier-casing-recovery-navigation` LSP example checks those
selection and rejection rows through definition, references, and
prepare-rename where the operation has an observable unsupported-symbol
result. The `identifier-casing-handler-binding-navigation` LSP example checks
the same operation set for invalid handler context and operation-clause
bindings. Focused language-service tests provide binding-form coverage for
parameters, result bindings, local bindings, pattern bindings, satisfy
candidate bindings, and handler bindings.

Recovery records remain quarantined. They do not enter normal workspace or
package symbol indexes, direct-dependency lookup, standard-prelude lookup,
cross-import visibility, exact-companion privilege, lowering, or backend
artifacts. LSP rename continues to return no edits for recovery selections;
repair rename remains proposal scope.

## Completion

This slice is complete for shared language-service and LSP definition,
references, and prepare-rename recovery navigation. It does not complete
module identity casing, repair rename, MCP rename mapping, or deferred
artifact consumers.
