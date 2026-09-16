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
| Direct fields and supported repeated payloads resolve to the selected workspace schema in canonical order. | `workspace_schema_references_cover_direct_and_repeated_composition_targets`, `references_return_workspace_schema_composition_locations_and_scope`, the MCP `references-workspace-schema-composition` case, and the LSP case of the same name |
| Written import paths and unique implicit leaf aliases share semantic composition resolution, while colliding leaf aliases and bare imports remain rejected. | `schema_composition_resolves_workspace_import_leaf_aliases_before_collision_checks`, `workspace_schema_references_cover_direct_and_repeated_composition_targets`, `colliding_implicit_schema_import_aliases_are_order_independent`, and the MCP `references-workspace-schema-composition` case |
| Visibility, shadowing, same-named workspace schemas, and exact companion access preserve identity boundaries. | `workspace_schema_references_include_exact_companion_private_qualified_uses`, `references_keep_same_named_workspace_schema_composition_identity`, and `references_keep_workspace_schema_identity_visibility_and_companion_boundaries` |
| Ordinary-type collisions, unresolved paths, and unrelated lexical matches are excluded. | `schema_composition_resolves_workspace_import_leaf_aliases_before_collision_checks`, `references_return_workspace_schema_composition_locations_and_scope`, and the MCP `references-workspace-schema-composition` case |
| Alias traversal and module-qualifier selections remain unsupported; package schemas, recovery records, and casing-neutral selections retain the shared successful-empty boundary. | `workspace_schema_references_keep_schema_specific_unsupported_selections_empty`, `package_composition_does_not_bind_same_named_workspace_schema`, `dependency_composition_with_matching_source_identity_stays_isolated`, and `references_reject_recovery_package_and_unsupported_symbols` |
| Anonymous and descendant sources do not widen navigation scope. | `references_keep_anonymous_sources_isolated_for_workspace_schema_selections` and `references_keep_descendant_package_sources_isolated_for_workspace_schema_selections` |
| Stable-capture exhaustion returns `snapshot_changed` without success-only locations or scope. | `references_project_capture_exhausts_retries_for_workspace_schema_selection` |
