---
role: implementation-record
update-when: The MCP `references` tool workspace schema-reference boundary, saved navigation capture contract, reference location shape, or executable MCP schema-reference evidence changes.
---

# MCP Saved Workspace Schema References

This record preserves the completed proposal that widened the saved MCP
`references` tool to selected-project workspace schema uses in `decode` and
`encode` expressions.

Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md)
and the checked `references-workspace-schema` executable MCP case.

## Completed Boundary

The selected symbol must be a schema declaration owned by the inferred saved
project. Selection follows the same module import, visibility, exact
test-companion, and shadowing rules used by definition lookup.

The result contains only schema path-leaf occurrences in `decode` and
`encode` expressions in the selected project's captured owned sources. It
includes local, imported, and module-qualified forms that resolve to the same
workspace schema. It excludes module qualifiers, the schema declaration, and
same-spelled paths that resolve to another declaration. It retains sorted
canonical `file:` locations and project-wide scope metadata.

This slice does not add package-schema references, schema composition or
public schema-alias references, package or workspace public-alias traversal,
pagination, declaration inclusion, recovery symbols, casing-neutral
selection, transitive dependency references, or rename behavior.

## Completion Evidence

| Behavior | Evidence |
| --- | --- |
| Local, imported, and module-qualified schema operation leaves resolve to the selected workspace schema. | `workspace_schema_references_cover_local_imported_and_qualified_operations` in `veln-language-service` tests and `references_return_workspace_schema_operation_locations_and_scope` in `veln-mcp` server tests |
| Import visibility, exact companion access, and shadowing preserve the definition-lookup identity boundary. | `workspace_schema_references_preserve_import_visibility_and_shadowing`, `workspace_schema_references_include_exact_companion_private_qualified_uses`, and `references_keep_workspace_schema_identity_visibility_and_companion_boundaries` |
| Same-spelled functions, types, constructors, values, fields, operations, strings, comments, and schema uses resolving elsewhere are excluded. | `workspace_schema_references_exclude_collisions_and_unsupported_selections` and `references_keep_workspace_schema_identity_visibility_and_companion_boundaries` |
| Package schemas and other unsupported selections keep successful empty results. | `references_reject_recovery_package_and_unsupported_symbols` |
| Anonymous-source and out-of-project single-file isolation remain unchanged for schema selections. | `references_keep_anonymous_sources_isolated_for_new_symbol_classes` and `references_use_single_file_scope_for_sources_outside_selected_projects` |
| Stable capture retry exhaustion for a workspace schema selection returns `snapshot_changed` without success-only references or scope metadata. | `references_project_capture_exhausts_retries_for_workspace_schema_selection` |
| The stdio executable specification demonstrates workspace schema references through the public MCP surface. | `references-workspace-schema` executable MCP case |
