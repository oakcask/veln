---
role: implementation-record
update-when: The MCP `references` tool workspace symbol admission boundary, saved navigation capture contract, reference location shape, or executable MCP references evidence changes.
---

# MCP Saved Workspace Symbol References

This record preserves the completed proposal that widened the MCP
`references` tool from workspace functions to the shared language-service
reference sets for non-recovery workspace types, constructors, value bindings,
handler context parameters, and handler operation clause parameters.

Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md)
and the checked
`../../../examples/specification/mcp/references-workspace/` executable case.
The focused MCP adapter tests cover the accepted symbol classes, identity and
shadowing boundaries, unsupported classes, recovery exclusion, package
exclusion, anonymous single-file isolation, coordinate boundaries, saved
capture retry exhaustion for project and anonymous navigation scopes, and
dependency resource capacity state preservation.

## Completed Boundary

`references` uses the existing saved-project capture, workspace-relative path,
one-based Unicode-scalar coordinate, canonical `file:` URI, deterministic
ordering, scope metadata, and dependency-resource admission machinery. It
returns reference-only locations for these non-recovery workspace symbols:

- functions;
- types;
- constructors;
- value bindings;
- handler context parameters;
- handler operation clause parameters.

The tool returns an empty `references` array for valid positions that select
schemas, effects, handlers, effect operations, recovery records, package-backed
symbols, unsupported occurrences, or no symbol. It does not change the tool
input schema, result schema, pagination model, workspace-function behavior,
or package-resource behavior.

## Completion Evidence

| Behavior | Evidence |
| --- | --- |
| Workspace types, constructors, value bindings, handler context parameters, and handler operation clause parameters are admitted by the MCP adapter. | `references_resolve_supported_workspace_symbol_classes` in `veln-mcp` server tests |
| Anonymous single-file reference lookup for newly admitted symbol classes remains isolated from other saved sources. | `references_keep_anonymous_sources_isolated_for_new_symbol_classes` |
| Recovery, package-backed, schema, effect-operation, unsupported, and absent selections succeed with empty references. | `references_reject_recovery_package_and_unsupported_symbols` and existing recovery/path tests |
| Reference coordinates keep CRLF, Unicode-scalar, token-end, and invalid-position boundaries. | `references_preserve_unicode_coordinates_and_token_end_exclusion` and existing schema coordinate tests |
| Stable capture changes during `references` exhaust bounded retries without success-only fields, dependency resource publication, or workspace selection changes. | `references_project_capture_exhausts_retries_after_owned_source_changes` and `references_anonymous_capture_exhausts_retries_after_requested_source_changes` |
| Dependency resource capacity and admission are operation-atomic for `references`. | `saved_project_capacity_failures_match_advertised_result_schemas`, `dependency_resource_capacity_is_atomic`, and successful saved-project admission tests |
| The stdio executable specification demonstrates the implemented symbol boundary. | `examples/specification/mcp/references-workspace/` |
