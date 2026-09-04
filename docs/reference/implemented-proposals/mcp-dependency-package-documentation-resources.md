---
role: implementation-record
update-when: MCP direct-dependency package-documentation resource admission, rendering, URI rejection, retention, capacity behavior, executable MCP evidence, or current MCP and package-documentation specifications change.
---

# MCP Dependency Package Documentation Resources

This record preserves the completed proposal history for publishing retained
direct-dependency package-documentation Markdown resources through MCP. Current
behavior is specified by [MCP resources](../../specification/mcp.md#resources)
and [Package Documentation Catalogs](../../specification/package-documentation.md).
The remaining package-backed definition-link work stays under
[MCP Package Definition Documentation Links](../../proposals/mcp-package-documentation-resources.md).

## Completed Boundary

Successful `check_project`, `definition`, and `references` operations admit
valid direct-dependency snapshots through the existing retained-package
boundary. The server generates the transport-independent
package-documentation result from the same snapshot and parsed manifest used
for admission. A successful result publishes one listed index resource and
exact linked module and declaration resources. A failed result publishes one
listed status resource instead. Source resources remain readable in either
case.

Dependency documentation resources use the package-documentation renderer's
canonical `veln-doc:` URIs, Markdown media type, names, titles, descriptions,
and allowlisted metadata. Module and declaration resources are readable only
by exact URI. They are advertised through the shared
`resources/templates/list` forms and are not eagerly listed by
`resources/list`.

The retained key is the package identity plus package snapshot digest.
Repeating the same identity and digest adds no resource state. A new digest
for the same identity coexists with older snapshots until server shutdown. If
admitting a new snapshot would exceed retained-package capacity, the operation
returns `resource_capacity`, publishes no source or documentation resources
for that rejected snapshot, and preserves the previous resource state.
Workspace refreshes and later dependency file changes do not remove or mutate
already admitted documentation resources.

This slice does not attach documentation URIs to package-backed definition
results, add package search, change `veln doc`, or change the
package-documentation catalog identity or renderer contract.

## Completion Evidence

| Contract | Evidence |
| --- | --- |
| Successful saved-project operations publish a dependency documentation index and exact linked module and declaration resources. | `dependency-package-documentation-resources` executable MCP case and `successful_dependency_documentation_resources_round_trip_from_rendered_result` |
| Status-only documentation failures list and read only the status resource while preserving source resources. | `dependency-package-documentation-resources` and `failed_dependency_documentation_publishes_only_status_resource` |
| Listed metadata uses renderer-provided names, titles, descriptions, media type, and URI byte ordering with existing resources. | `dependency-package-documentation-resources` and `saved_project_dependency_resources_list_with_complete_sorted_metadata` |
| Wrong-snapshot, wrong-documentation-digest, unpublished, missing, malformed, and noncanonical documentation URIs fail with `resource_not_found` without normalization or filesystem fallback. | `dependency-package-documentation-resources` and `dependency_documentation_resource_rejections_are_exact` |
| Repeated admission deduplicates by identity and snapshot digest, while later snapshots for the same identity coexist and keep their original bytes. | `dependency_resource_admission_deduplicates_and_preserves_state_on_failures` and `dependency_documentation_snapshots_coexist_and_remain_retained` |
| Retained-package capacity failure is atomic and publishes no documentation resource from rejected snapshots. | `dependency_resource_capacity_is_atomic` |
