---
role: proposal
update-when: The MCP references tool schema, saved navigation capture, shared workspace-function reference result, or planned adapter evidence changes.
---

# MCP Saved Workspace Function References

## Summary

Expose the shared language service's current workspace-function reference
results through `veln mcp`. This is an adapter slice of the broader agent
language-services plan, not a request to expand language resolution or the
shared navigation symbol set.

## Implemented Prerequisites

- [MCP Workspace Projects, Diagnostics, And Definitions](../specification/mcp.md)
  specifies saved project selection, stable capture, coordinates, schemas, and
  the existing definition adapter.
- [Editor Support](../specification/editor-support.md) specifies the shared
  language-service function reference result consumed by this adapter.
- [Identifier Casing](../reference/implemented-proposals/identifier-casing.md)
  records the accepted-source name-class boundary used by that shared result.

Explicit import aliases, repair candidate isolation, and MCP rename mapping do
not affect this adapter slice.

## Scope

| Included | Excluded |
| --- | --- |
| A checked `references` input with `source`, `line`, and `column`. | `include_declaration`, page size, continuation cursors, and retained cursor state. |
| Selected-project and anonymous single-file capture using existing navigation selection. | Dependency and standard-library reference search or virtual locations. |
| Canonical `file:` locations for current project-owned function reference sites in deterministic order. | New symbol kinds or broader definition coverage. |
| Explicit project or single-file scope metadata, including whether the result is project-wide. | Changes to language name resolution, callable classification, lowering, LSP behavior, or shared navigation selection. |
| Empty success for a valid position without supported function reference search. | Exhaustive enumeration of expressions that can produce, store, or shadow callable values. |
| Existing path, coordinate, schema, and stable-capture failures. | Pagination, resource lifetime, documentation tools, plugin work, formatting, and rename. |

An independently reproducible language-resolution or LSP-navigation defect is
separate work unless it prevents one acceptance row from using the current
shared function result.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Request references at a project-owned function call in a selected project. | Return only that function's project-owned reference sites as sorted canonical `file:` locations, plus project scope metadata. | One MCP stdio case with a declaration, recursive call, ordinary call, and unrelated ambiguous constructor call. |
| Request references at the unrelated ambiguous constructor call. | Return an empty reference list because constructor reference search is outside this slice. | The same MCP stdio case. |
| Request references for an accepted source outside a selected project's owned-source set. | Analyze only that source and report single-file scope with `project_wide: false`. | One descendant-boundary or anonymous-source case. |
| Supply an invalid position or schema-invalid coordinate. | Return `invalid_position` for an unaddressable positive coordinate and protocol invalid params for a non-integer coordinate. | MCP stdio and schema cases. |
| Replace a captured source identity or bytes during the operation. | Return `snapshot_changed` without partial reference locations. | Existing navigation stable-capture harness extended to `references`. |
| List MCP tools after initialization. | Advertise the checked `references` input and result schemas. | Existing workspace-lifecycle tool-list case. |

## Completion

The proposal is complete when all six rows pass and the implemented contract
is promoted to the MCP specification and executable-example routes. Broader
agent language-service work remains in
[Agent Language Services](agent-language-services.md) and must be extracted as
another bounded proposal before becoming Ready.
