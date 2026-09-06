---
role: implementation-record
authority: supporting
update-when: The completed MCP package-documentation tool scope, checked schemas, executable MCP evidence, or current MCP specification changes.
---

# MCP Package Documentation Tools

## Completed Scope

The completed slice extends the existing `search_docs` and `read_doc` tools to
retained embedded standard-library and admitted direct-dependency package
documentation. Current behavior is specified by
[MCP Workspace Projects And Navigation](../../specification/mcp.md).

## Scope

| Included | Excluded |
| --- | --- |
| `language`, `stdlib`, `package`, and `all` scopes on `search_docs`. | Workspace-package documentation, arbitrary generated documentation output, source resources, status search candidates, fuzzy search, pagination, and persistent indexes. |
| Search candidates from successful retained package-documentation indexes, modules, and declarations. | Regenerating package documentation during search or read. |
| Exact `read_doc` access to retained package-documentation index, status, module, and declaration resources. | Fallback reads for unknown, noncanonical, wrong-snapshot, wrong-documentation-digest, or unpublished URIs. |
| Atomic publication, deduplication, digest coexistence, status-only read boundaries, capacity preservation, and lifecycle evidence. | Dependency reference navigation, broader definition navigation, rename, and client plugins. |

## Completion Evidence

| Case | Evidence |
| --- | --- |
| Tool schemas advertise the expanded search scope and exact read shape. | Checked schema tests and MCP tool-list specification cases. |
| Package field ranking, scope selection, URI byte ordering, deduplication, snapshot coexistence, and lifecycle retention hold for retained package documentation. | `veln-mcp` package documentation tool tests. |
| Exact package-documentation reads match `resources/read`, while source, unknown, wrong-digest, and unpublished URIs return `resource_not_found`. | `veln-mcp` exact-read and rejection tests. |
| Stdio checks exercise package-scope search and exact package-documentation `read_doc` for successful and status-only dependency documentation. | `examples/specification/mcp/dependency-package-documentation-resources/`. |

