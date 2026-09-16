---
role: implementation-record
update-when: The MCP references tool workspace schema composition-reference boundary, schema composition source surface, or executable evidence changes.
---

# MCP Saved Workspace Schema Composition References

This record preserves the completed expansion of the MCP `references` tool to
directly resolved schema-composition leaves for selected workspace schemas.
Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md)
and the checked `references-workspace-schema-composition` executable MCP case.

## Completed Boundary

The shared language-service result joins resolved composition leaf spans to
the workspace schema declaration identity. It includes direct fields and both
supported repeated-payload spellings. Full written import paths and implicit
leaf import aliases use the same visibility and exact-companion rules as
schema definition lookup. Existing `decode` and `encode` references remain in
the same canonical result.

The MCP adapter exposes the expanded shared result without a wire-schema
change. Project selection, anonymous and descendant isolation, stable-capture
failure, and success-only scope metadata retain their existing behavior.

## Scope Boundary

This slice does not add schema-alias identity or alias-chain references,
package-schema references, declaration inclusion, package-source locations,
pagination, recovery or casing-neutral selection, transitive-dependency
references, rename behavior, or MCP schema expansion.

## Completion Evidence

| Behavior | Evidence |
| --- | --- |
| Direct fields and supported repeated payloads resolve to the selected workspace schema in canonical order. | `workspace_schema_references_cover_direct_and_repeated_composition_targets`, `references_return_workspace_schema_composition_locations_and_scope`, and `references-workspace-schema-composition` |
| Import paths, implicit leaf aliases, rejected bare imports, visibility, shadowing, and exact companion access preserve identity boundaries. | Focused language-service schema-reference tests and MCP project-scope tests |
| Alias traversal, package schemas, recovery and invalid-casing selections, unrelated lexical matches, and module qualifiers remain unsupported or excluded. | Language-service unsupported-selection tests, MCP unsupported-symbol tests, and executable boundary requests |
| Anonymous and descendant sources do not widen navigation scope. | `references_keep_anonymous_sources_isolated_for_workspace_schema_selections` and `references_keep_descendant_package_sources_isolated_for_workspace_schema_selections` |
| Stable-capture exhaustion returns `snapshot_changed` without success-only locations or scope. | `references_project_capture_exhausts_retries_for_workspace_schema_selection` |
