---
role: proposal
update-when: MCP package-documentation resources, documentation tool schemas, package search fields, retained snapshot behavior, or planned package-tool evidence changes.
---

# MCP Package Documentation Tools

## Summary

Extend the existing `search_docs` and `read_doc` tools to the embedded
standard-library and admitted direct-dependency documentation resources. This
slice gives an agent a bounded route to discover and read package APIs without
changing package-documentation generation, resource publication, or snapshot
identity.

## Cleared Blocker

The required foundations are implemented and specified:

- [MCP Workspace Projects And Navigation](../specification/mcp.md) defines the
  language-only documentation tools and publishes immutable standard-library
  and direct-dependency package-documentation resources.
- [Package Documentation Catalogs](../specification/package-documentation.md)
  defines the structured package, module, declaration, documentation,
  signature, and keyword fields that search can index without parsing rendered
  Markdown.
- The retained-package boundary already makes standard-library documentation
  available at startup and admits direct-dependency documentation atomically
  with saved-project analysis.

The umbrella proposal previously deferred package search until these resource
and tool contracts existed. They now provide a finite adapter slice with no
dependency on rename, pagination, broader navigation, or client plugins.

## Scope

| Included | Excluded |
| --- | --- |
| `stdlib`, `package`, and `all` scopes on the existing `search_docs` tool. | Workspace-package documentation and arbitrary `veln doc` output. |
| Search candidates from successful retained package-documentation indexes, modules, and declarations. | Status resources as search candidates and source resources of any kind. |
| Exact `read_doc` access to every retained package-documentation index, status, module, and declaration resource. | New resource types, package admission paths, catalog generation, or Markdown rendering. |
| Existing query bounds, normalization, ranking, result shape, and URI ordering across language and package candidates. | Fuzzy matching, stemming, locale-sensitive ranking, pagination, and persistent indexes. |
| Atomic candidate publication, snapshot coexistence, capacity preservation, and stdio evidence. | Dependency references, broader definition, recovery navigation, rename, and client plugins. |

The current `language` scope and language-resource reads remain unchanged. The
tools consume the same retained structured catalogs and rendered resources as
the MCP resource routes. They do not analyze package source or regenerate
documentation during search or read.

## Search Contract

`search_docs` keeps its required `query`, optional `scope`, optional `limit`,
and existing result fields. The default scope remains `language`. The accepted
scopes become:

| Scope | Candidate set |
| --- | --- |
| `language` | The existing checked language topics. |
| `stdlib` | Successful package-documentation candidates for the retained `std` snapshot. |
| `package` | Successful package-documentation candidates for retained non-standard snapshots. |
| `all` | The union of the other three candidate sets. |

A successful package-documentation result contributes its index, modules, and
declarations. A status-only result contributes no search candidate. Each
candidate appears once for its exact resource URI. Different retained snapshot
or documentation digests remain distinct candidates even when their package,
module, or declaration names are equal.

Package candidates use the structured catalog rather than reparsing rendered
Markdown. The first matching tier determines rank:

1. The complete normalized query equals the candidate identifier or title.
2. The candidate identifier or title starts with the complete normalized query.
3. Every token occurs in the title, name, or package keywords.
4. Every token occurs in the description or declaration signature.
5. Every token occurs in the catalog-owned documentation text.

For an index, the identifier is the package identity and the name is the
package name when present. For a module or declaration, the identifier is its
catalog ID and the name is its catalog name. The title and description are the
same values published in the corresponding rendered resource metadata.
Package keywords apply to every candidate from that package. A declaration
signature is searchable only for that declaration. Catalog-owned documentation
text includes module or declaration documentation, constructor documentation,
and contract text. It excludes source code, doctest code, expected output,
diagnostic text, physical paths, and rendered Markdown decoration.

The existing NFC normalization, pinned full Unicode default case fold, query
bounds, 160-scalar excerpt limit, truncation flags, result limit, and no-cursor
behavior apply to every scope. The excerpt comes from the first matching field
in the winning tier and preserves the original catalog text. Equal-rank results
sort by exact resource URI UTF-8 bytes across the complete selected scope.

## Read And State Contract

`read_doc` accepts every exact retained package-documentation index, status,
module, and declaration URI in addition to the existing language URIs. Success
returns the renderer's URI, name, title, optional description, and media type,
plus the same complete Markdown text as the corresponding `resources/read`
route. Hidden module and declaration resources are readable even though
`resources/list` does not enumerate them.

An unknown, noncanonical, wrong-snapshot, wrong-documentation-digest, or
unpublished package-documentation URI returns `resource_not_found` without
partial content, normalization, regeneration, or filesystem fallback.

The standard-library candidate set is available after initialization. A
successful direct-dependency admission publishes its resource and search state
atomically. Repeated admission of the same retained key adds no candidates. A
different digest for the same identity coexists with the older candidates and
reads until shutdown. A status-only documentation result adds only its exact
`read_doc` route. A capacity or capture failure adds no package candidates or
read routes and preserves all earlier tool results and resources. Workspace
refresh and later filesystem changes do not remove or mutate retained package
search candidates or reads.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| List tools after initialization. | The checked `search_docs` schema accepts all four scopes, the `read_doc` schema remains exact, and both declarations retain their existing bounds and result shapes. | Schema freshness and exact stdio tool-list cases. |
| Search `stdlib` for a package, module, declaration, signature, keyword, or documentation term. | Only successful retained `std` candidates match, using the declared rank, excerpt, limit, and URI-order rules. | Table-driven standard-library field and rank matrix plus a stdio search case. |
| Search `package` before and after a successful dependency admission. | The dependency has no candidate before admission; its index, modules, and declarations become visible atomically afterward. | Saved-project admission transition cases. |
| Search `language`, `stdlib`, `package`, and `all` for overlapping terms. | Each scope contains exactly its declared candidate set; `all` merges equal-rank results by URI bytes without duplicate URIs. | Cross-scope candidate and ordering matrix. |
| Admit the same dependency again or admit a later snapshot for the same identity. | Repeated admission adds no duplicates; a new digest adds distinct immutable candidates while old candidates remain searchable. | Deduplication and snapshot-coexistence cases. |
| Produce status-only package documentation. | The failed result is absent from search, but its exact status URI remains readable through `read_doc`; no unpublished index, module, or declaration URI is readable. | Generation-failure search and exact-read boundary cases. |
| Read retained index, status, module, and declaration URIs through both routes. | `read_doc` metadata and complete Markdown bytes equal the corresponding resource metadata and `resources/read` content. | Route-equality table covering listed and hidden resources. |
| Read a noncanonical, unknown, wrong-snapshot, wrong-documentation-digest, or unpublished URI. | The tool returns `resource_not_found` with no content or state change. | Package-documentation URI rejection table and stdio domain-failure cases. |
| Reach package capacity or exhaust stable capture during admission. | No rejected candidate or read route appears, and every earlier search result and read remains unchanged. | Capacity and capture-failure state-preservation cases. |
| Refresh the workspace or change dependency files after admission. | Retained candidates, ordering, metadata, and read bytes remain unchanged until shutdown. | Lifecycle state-preservation cases. |
| Search with invalid query, scope, or limit input. | Existing invalid-params behavior applies without clamping, partial results, or resource-state changes. | Checked-schema and protocol parameter table for every scope. |

## Completion

This proposal is complete when every acceptance row has executable MCP
coverage, a stdio specification case checks package search and exact package
reads, and the MCP specification states the implemented scope and lifecycle
boundaries. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
