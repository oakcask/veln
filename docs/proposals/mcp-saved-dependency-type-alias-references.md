---
role: proposal
update-when: The MCP references tool schema, saved direct-dependency type-alias navigation boundary, or planned dependency type-alias reference evidence changes.
---

# MCP Saved Direct-Dependency Type-Alias References

## Summary

Expose selected-project references to public type aliases from exported
direct-dependency modules through the existing MCP `references` tool.

## Readiness

This slice is ready. The current language service already resolves package
types, public type aliases, and the selected project's saved source set. The
MCP adapter already returns project-scoped references for direct-dependency
type declarations and public function aliases. This proposal extends that
boundary to one additional declaration class. It does not depend on reference
pagination, schema navigation, standard-library alias support, or plugin
packaging.

## Contract

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select a visible public type alias whose target resolves to a type declaration in the same retained direct dependency. | Return the consumer's references to that alias in canonical order with project scope and `project_wide: true`. | Language-service identity cases, MCP adapter cases, and an executable MCP stdio case. |
| Use the alias in a type annotation, type argument, return type, type-alias right-hand side, or constructor qualifier. | Include the alias leaf for every occurrence that ordinary name resolution binds to the selected public alias. | A table-driven language-service case and exact executable reference ranges. |
| Reach the exported module through its written module path or its import alias. | Resolve both spellings to the same public type-alias identity. | Qualified-path and import-alias-qualified cases. |
| Give the alias and its target type the same or different spellings and select each identity. | Alias selection returns only alias-bound occurrences. Target selection does not absorb occurrences bound to the alias. | Paired alias and target selection cases. |
| Encounter the same spelling in another package, another selected project, a package implementation body, or an unrelated workspace declaration, value, field, string, or comment. | Exclude that occurrence from the result. Return only canonical workspace `file:` locations from the inferred selected project. | Collision, project-isolation, package-body-exclusion, and lexical-noise cases. |
| Select a private alias, an alias from a non-exported module, a transitive-dependency alias, an unresolved or wrong-kind alias target, an invalid-casing alias record, or an unsupported alias chain. | Succeed with an empty `references` array and do not reinterpret the selection as the target type. | Table-driven unsupported-selection cases through the language service and MCP adapter. |
| Exhaust stable-capture retries while resolving a supported alias selection. | Return `snapshot_changed` without reference locations, success-only fields, or partial package-resource admission. | MCP capture-mutation test. |

The existing `references` input and result schemas are sufficient. This slice
does not add a domain error or wire field.

## Scope Boundary

This proposal does not add standard-library type-alias references, public
schema-alias references, alias-chain traversal, declaration inclusion,
package-source reference locations, recovery or casing-neutral selection,
transitive-dependency references, pagination, rename behavior, or MCP schema
expansion.

## Verification And Completion

Implementation must add:

- a `references-dependency-type-alias` case under
  `examples/specification/mcp/` that checks the observable result and boundary
  rows above;
- focused language-service tests for package type-alias identity and reference
  collection; and
- MCP server tests for saved-project scope, unsupported selections, descendant
  project isolation, and stable-capture failure.

The proposal is complete when those checks pass and
[MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md)
states the implemented public type-alias boundary. Move the completed record to
`../reference/implemented-proposals/` and remove this page from the proposal
catalog at that time.
