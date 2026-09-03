---
role: proposal
update-when: The checked language-reference catalog, published MCP language resources, documentation tool schemas, Unicode search contract, or planned language-tool evidence changes.
---

# MCP Language Reference Tools

## Summary

Add model-controlled `search_docs` and `read_doc` tools for the checked
language-reference topics already published through MCP resources. This slice
lets an agent discover a bounded topic result and read its complete Markdown
without adding package catalogs, virtual sources, or snapshot state.

## Scope

| Included | Excluded |
| --- | --- |
| Checked v1 input and result schemas for `search_docs` and `read_doc`. | Package and standard-library documentation or an `all` search scope. |
| Deterministic search over the checked language-topic catalog. | Workspace analysis, dependency loading, and persistent indexes. |
| Exact reads of the existing language index and topic resources. | New resource kinds, resource templates, pagination, and subscriptions. |
| Unicode-normalized matching, ranking, excerpts, and bounds. | Fuzzy matching, stemming, locale-sensitive behavior, and relevance tuning from usage data. |
| Protocol, tool-result, lifecycle, and stdio evidence. | Navigation expansion, rename, plugins, and conformance-manifest completion. |

The tools consume the same checked catalog, digest, and rendered resource set
as ordinary MCP resource reads. They do not execute the source grammar,
regenerate Markdown, inspect development documentation, or analyze a
workspace.

## Tool Contracts

`search_docs` accepts an object with required string `query`, optional `scope`,
and optional `limit`. `scope` defaults to `language`, and this slice accepts
only `language`. `limit` defaults to 10 and accepts JSON integers from 1
through 50. The query must contain from 1 through 256 Unicode scalars before
normalization and at least one non-whitespace scalar after normalization.
Unknown fields, `null`, non-object input, unsupported scopes, non-integer
limits, and values outside these bounds are invalid params; values are not
clamped.

A successful search returns the effective scope and an ordered `results`
array. Each result contains the exact topic resource URI, topic title, topic
summary, a matching excerpt, and independent `prefix_truncated` and
`suffix_truncated` flags. The language index is a read route, not a search
candidate. The result count is at most the effective limit. Search has no
cursor and returns an empty array when no topic matches.

`read_doc` accepts an object containing one exact `uri`. This slice accepts the
checked language index URI and checked language topic URIs. Success returns
the listed resource's URI, name, title, optional description, and media type,
plus the same complete Markdown text as `resources/read`. Unknown fields,
`null`, non-object input, and a missing or non-string URI are invalid params. A
syntactically valid but unknown, noncanonical, wrong-digest, non-language, or
unknown-topic URI returns an MCP tool error with domain code
`resource_not_found` and no content.

Both declarations derive from checked schemas in the existing MCP v1 bundle.
Initialization and `tools/list` advertise them with the existing tools.

## Search Contract

Search normalizes the query and each searched field to NFC, applies full
Unicode default case folding under the workspace's pinned Unicode 17 contract,
trims Unicode whitespace, and splits the query on one or more Unicode
whitespace scalars. A topic matches only when every query token occurs in at
least one searchable field. Searchable fields are topic identifier, title,
keywords, summary, and body. Grammar and example source blocks are not searched
in this slice.

The first matching tier determines rank:

1. the complete normalized query equals the identifier or title;
2. the identifier or title starts with the complete normalized query;
3. every token occurs in the title or keywords;
4. every token occurs in the summary; or
5. every token occurs in the body.

Equal-tier results sort by resource URI UTF-8 bytes. One URI appears at most
once. The excerpt comes from the first match in the first field of the winning
tier, using field order identifier, title, keywords in catalog order, summary,
then body. It preserves the original field text, contains at most 160 Unicode
scalars, and includes the complete source span corresponding to the first
matched token when that span itself is no longer than the excerpt limit. The
truncation flags report whether original field content was omitted before or
after the excerpt.

## State And Failure Boundaries

The search candidates, result URIs, read metadata, and read bytes are fixed by
the checked language-reference artifact for the server lifetime. Workspace
refresh and project analysis do not change them. Invalid input and domain
failures do not mutate server state and do not return partial search results or
partial content.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| List tools after initialization. | Both declarations exactly match the checked v1 schemas and coexist with the implemented tools and resources. | Schema freshness test and exact stdio tool-list case. |
| Search by exact identifier or title and by prefix. | Exact results precede prefix results, equal-tier results use URI byte order, and each topic appears once. | Table-driven rank and tie cases. |
| Search with mixed case, decomposed Unicode, compatibility folds, or repeated Unicode whitespace. | NFC and the pinned full default case fold produce the same matches as their normalized forms. | Unicode normalization and case-fold vectors shared with the portable Unicode contract. |
| Search across title, keywords, summary, and body. | All tokens must match; the winning tier and first matching field determine a bounded excerpt and accurate truncation flags. | Field-tier, token-intersection, excerpt-boundary, and scalar-count cases. |
| Search with no match or a restrictive limit. | The result is empty or contains the first ordered results up to the effective limit, with no cursor. | Empty-result and limit-boundary cases. |
| Supply an empty, whitespace-only, oversized, nullable, malformed, or unknown-field search input. | The request fails as invalid params without clamping or a partial result. | Checked-schema and protocol parameter table. |
| Read the language index and a topic through both routes. | `read_doc` metadata equals the listed resource metadata, and its URI, media type, and complete text equal `resources/read`. | Route-equality tests and exact stdio reads. |
| Read an unknown, noncanonical, wrong-digest, non-language, or unknown-topic URI. | The tool returns `resource_not_found` with no content and performs no filesystem fallback. | URI rejection table and stdio domain-failure case. |
| Refresh the workspace and analyze a project between searches and reads. | Search order, result URIs, metadata, and read bytes remain identical. | MCP lifecycle state-preservation test. |

## Completion

This proposal is complete when every acceptance row passes and the MCP
specification and executable examples state the implemented language-only
tool behavior. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
