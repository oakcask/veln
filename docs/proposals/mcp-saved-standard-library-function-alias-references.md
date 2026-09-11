---
role: proposal
update-when: The public standard-library function-alias API, saved MCP standard-library navigation boundary, reference result schema, or planned standard-library function-alias evidence changes.
---

# MCP Saved Standard-Library Function-Alias References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of a
public function alias from an exported embedded standard-library module.
This is a separate saved-navigation slice from direct-dependency public
function-alias references.

## Readiness

This proposal is blocked until an accepted standard-library API proposal adds
a meaningful exported public function alias with checked bundle evidence. Do
not add a placeholder standard-library alias only to test navigation.

## Scope

The selected symbol must be a public function alias in an exported embedded
standard-library module. Selection must follow the same saved-project
implicit-prelude, explicit `std` import, visibility, and shadowing rules used
by definition lookup.

The result will contain only occurrences that resolve to the selected alias in
the selected project's captured owned sources. It will include qualified
calls, qualified function-value occurrences, and accepted bare implicit
prelude forms. It will exclude the alias declaration, the alias target,
occurrences of the target function, standard-library source bodies, and equal
spellings that resolve to a different declaration. It will retain the existing
sorted `file:` locations and project-wide scope metadata.

Selecting the target function remains a distinct query and does not include
uses of its public alias. This slice does not add direct-dependency reference
behavior, public type-alias or public schema-alias references, alias-chain
traversal, pagination, declaration inclusion, package-source reference
locations, recovery symbols, transitive dependency references, casing-neutral
selection, rename behavior, or a new MCP schema.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select one exported embedded standard-library public function alias through accepted implicit-prelude and `std`-qualified forms. | Return each resolved alias occurrence once in canonical location order with project-wide scope. | Language-service standard-library alias-identity cases and one executable MCP specification case. |
| Select the alias target or use the target function directly. | Keep target-function references separate from alias references in both directions. | Language-service target-and-alias identity matrix and MCP adapter cases. |
| Reuse the alias spelling for another module, function, type, constructor, value, field, operation, string, comment, or dependency package identity. | Exclude every occurrence that does not resolve to the selected standard-library alias. | Language-service collision table and MCP boundary cases. |
| Select a private alias, an alias from a non-exported module, an alias chain, a public type or schema alias, an invalid-casing alias, a recovery symbol, a package module segment, or another unsupported symbol class. | Return an empty `references` array without widening the supported symbol set. | Language-service visibility and unsupported-selection table plus MCP success-boundary cases. |
| Request the same selection from an anonymous source or a source outside the inferred selected project. | Preserve single-file isolation and return no standard-library alias references. | Descendant-project and unrelated-source isolation cases. |
| Change the selected project or package inputs during capture until retry exhaustion. | Return `snapshot_changed` without reference locations or scope metadata and without partial package resource admission. | Existing saved-reference capture and state-preservation harnesses extended with standard-library function-alias selection. |

## Completion

This proposal is complete when every acceptance row has checked evidence, the
MCP specification states the implemented standard-library public
function-alias boundary, and the completed proposal record identifies the
executable specification and unit-test routes. The implementation must then
remove this page from the active proposal catalog.
