---
role: implementation-record
authority: supporting
update-when: The MCP language-reference resource capability, checked Markdown renderer, executable resource cases, or current MCP and language-reference specifications change.
---

# MCP Language Reference Resources

The completed slice publishes the checked language-reference catalog through
`veln mcp` as immutable Markdown resources. Current behavior is specified by
[MCP Workspace Projects And Navigation](../../specification/mcp.md) and
[Language Reference Catalog](../../specification/language-reference-catalog.md).

## Completed Scope

- Added deterministic Markdown rendering over the checked catalog and digest.
- Published one index resource and one topic resource per checked catalog
  topic through `resources/list` and `resources/read`.
- Added exact URI lookup, complete non-paginated reads, strict parameter
  validation, and structured `resource_not_found` failures.
- Kept resource content independent of workspace discovery, refresh, and
  project analysis.
- Extended language-reference freshness validation to reject renderer and
  size-limit drift without adding SWI-Prolog to ordinary MCP builds.

## Completion Evidence

| Claim | Checked evidence |
| --- | --- |
| Initialization advertises immutable resource capability and list/read publish deterministic checked metadata, Markdown, and resolvable resource links. | `cargo test -p veln-mcp` and the `language-reference-resources` executable MCP specification case |
| Malformed parameters, exact URI lookup, noncanonical or unknown URIs, wrong digests, and unknown topics have the specified protocol failures. | `cargo test -p veln-mcp` and the `language-reference-resources` executable MCP specification case |
| Resource URIs, metadata, and bytes are stable across refresh and project analysis. | `cargo test -p veln-mcp` |
| Markdown rendering preserves catalog order and semantic content, enforces the byte limit, and excludes development provenance. | `cargo test -p veln-repo-language-reference` |
| Freshness validation detects catalog, catalog-digest, rendered-resource-digest, and size-limit drift while ordinary consumers use checked inputs. | `cargo test -p veln-repo-language-reference` and `cargo run -p veln-repo-language-reference -- . check-fresh` |

## Preserved Non-Goals

This slice does not add documentation tools, package resources, standard
library resources, source resources, pagination, subscriptions, runtime catalog
regeneration, navigation expansion, plugins, or conformance-manifest
completion.
