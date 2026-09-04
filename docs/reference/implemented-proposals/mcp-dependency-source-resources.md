---
role: implementation-record
update-when: MCP direct-dependency source resource admission, capacity, lifecycle, URI, metadata, or executable evidence changes.
---

# MCP Dependency Source Resources

This record preserves the completed proposal history for publishing captured
direct-dependency distribution sources through MCP resources. Current behavior
is specified by [MCP resources](../../specification/mcp.md#resources) and
checked by the `dependency-source-resources` executable MCP case plus focused
`veln-mcp` unit tests.

The completed slice admits valid direct-dependency package snapshots from
successful `check_project`, `definition`, and `references` saved-project
operations. Admission is atomic per operation. The retained key is package
identity plus package snapshot digest, so repeated captures deduplicate and a
new digest for the same identity coexists with older snapshots. The retained
package capacity is 256 snapshots including the embedded standard library, and
an exceeding operation returns `resource_capacity` without changing existing
resource state.

Each retained dependency distribution source is listed with its canonical
`veln-pkg:` URI, package-relative name, `Veln package source: {identity}: {path}`
title, and `text/x-veln; charset=utf-8` media type. Reads return the exact
retained UTF-8 source text and do not normalize rejected URIs or fall back to
the filesystem. Workspace refreshes and later dependency file changes do not
remove or mutate already admitted dependency resources.

## Completion Evidence

| Contract | Evidence |
| --- | --- |
| Successful saved-project operations admit valid direct dependencies. | `successful_saved_project_tools_admit_dependency_resources_until_shutdown` and `dependency-source-resources` |
| Identity-and-digest deduplication and same-identity digest coexistence hold. | `dependency_resource_admission_deduplicates_and_preserves_state_on_failures` and `successful_saved_project_tools_admit_dependency_resources_until_shutdown` |
| Invalid positions, failed operations, malformed parameters, and rejected resource reads preserve existing state. | `dependency_resource_admission_deduplicates_and_preserves_state_on_failures`, `resources_reject_malformed_params_and_unknown_uris`, and navigation failure tests |
| Capacity failure is atomic at the 256 snapshot limit. | `dependency_resource_capacity_is_atomic` |
| Listed dependency resources are sorted with other resources and read from retained bytes. | `saved_project_dependency_resources_list_with_complete_sorted_metadata`, `successful_saved_project_tools_admit_dependency_resources_until_shutdown`, and `dependency-source-resources` |
| Private and non-exported distribution sources are published while test-shaped sources are rejected. | `successful_saved_project_tools_admit_dependency_resources_until_shutdown` and `dependency-source-resources` |
