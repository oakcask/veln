---
role: proposal
update-when: The MCP references tool schema, saved workspace schema composition-reference boundary, schema composition source surface, or planned executable evidence changes.
---

# MCP Saved Workspace Schema Composition References

## Summary

Extend the existing MCP `references` tool to include selected-project schema
composition targets that resolve to a selected workspace schema. Keep the
current unpaginated result schema and project-owned source-location boundary.

## Readiness

This slice is ready. The language service already selects workspace schema
identities and returns their resolved `decode` and `encode` uses. The source
surface already resolves same-module and imported schema composition targets,
including supported schema field and repeated-payload positions. Saved project
capture, canonical reference ordering, and stable-capture failure handling are
also implemented. The existing MCP result schema can represent the additional
workspace locations without a wire change.

This slice does not depend on public schema-alias selection, package schema
navigation, pagination, recovery navigation, conformance completion, or plugin
packaging.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select a workspace schema used as a same-module composition target. | Return the target path leaf together with existing `decode` and `encode` uses in canonical location order with project scope and `project_wide: true`. | Language-service identity cases, MCP adapter cases, and an executable MCP stdio case. |
| Use the schema as a direct field target or as a supported repeated payload target. | Include every schema path leaf that ordinary schema composition resolution binds directly to the selected schema. | Table-driven language-service cases and exact executable reference ranges. |
| Reach a public workspace schema through a written import path or its implicit leaf import alias. | Resolve accepted qualified spellings to the same schema identity. Do not treat a written import as a bare schema import. | Qualified-path, import-alias-qualified, and rejected bare-use cases. |
| Select a private schema from its exact test companion. | Include composition targets from the exact companion without making the schema visible to unrelated modules or companions. | Language-service companion-boundary cases and MCP project-scope cases. |
| Encounter the same spelling in another schema, project, ordinary type, function, value, field, string, comment, module qualifier, or unresolved composition path. | Exclude every occurrence that does not resolve directly to the selected schema. | Collision, project-isolation, and lexical-noise cases. |
| Compose through a public schema alias, select the alias declaration or target leaf, or select a package schema, recovery symbol, or invalid-casing record. | Preserve the existing successful empty result for unsupported selection and alias traversal. Do not reinterpret an alias occurrence as a direct target reference. | Unsupported-selection cases through the language service and MCP adapter. |
| Request the same selection from an anonymous source or a source outside the inferred selected project. | Preserve single-file and descendant-project isolation without widening the navigation scope. | Anonymous-source and descendant-project MCP cases. |
| Exhaust stable-capture retries while resolving a supported composition selection. | Return `snapshot_changed` without reference locations or success-only scope metadata. | MCP capture-mutation test. |

## Scope Boundary

This proposal does not add schema-alias identity or alias-chain references,
package-schema references, declaration inclusion, package-source locations,
pagination, recovery or casing-neutral selection, transitive-dependency
references, rename behavior, or MCP schema expansion.

## Verification And Completion

Implementation must add:

- a `references-workspace-schema-composition` case under
  `examples/specification/mcp/` that checks the observable result and boundary
  rows above;
- focused language-service tests for composition-target identity, import,
  visibility, collision, alias, and companion boundaries; and
- MCP server tests for saved-project scope, unsupported selections, anonymous
  and descendant isolation, and stable-capture failure.

The proposal is complete when those checks pass and
[MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md)
states the implemented workspace schema composition-reference boundary. Move
the completed record to `../reference/implemented-proposals/` and remove this
page from the proposal catalog at that time.
