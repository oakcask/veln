---
role: proposal
update-when: The remaining package-navigation, cross-adapter-conformance, or client-plugin scope changes.
---

# Agent Language Services

This umbrella records only the unimplemented follow-up work for language
services. The implemented workspace, direct-dependency, and standard-library
navigation behavior is specified by [Editor Support](../specification/editor-support.md)
and [MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md).

## Remaining scope

The following work remains planned:

- standard-library and other dependency schema-alias navigation beyond the
  currently supported direct-dependency boundary;
- transitive-dependency navigation;
- recovery and casing-neutral reference navigation;
- remaining package definition and reference symbol classes;
- cross-adapter conformance evidence for saved navigation;
- Codex and Claude Code plugin packaging and client-native installation flows.

Each follow-up must define its own observable acceptance cases and executable
evidence before implementation. It must update the matching current
specification page and remove its completed scope from this inventory.

## Remaining acceptance model

The following rows are the umbrella's remaining planning contract. They are
not claims about the current implementation; each row remains incomplete until
its evidence is added to the named current specification and checked route.

| Requirement | Observable acceptance | Required evidence |
| --- | --- | --- |
| Declaration inclusion | A reference request with declaration inclusion returns only the declaration kinds explicitly admitted by the selected symbol class; package-source implementation uses remain excluded. | Paired language-service, LSP, and MCP cases over the same saved snapshot, including positive, private, and package-source boundaries. |
| Cross-adapter equality | LSP and MCP requests over the same saved project produce the same filesystem identities and normalized locations after declaration-policy and pagination normalization. | A paired adapter matrix with exact URI/range comparison, including Unicode coordinates, empty results, and an MCP continuation page. |
| Coordinate matrix | Empty, LF, CRLF, terminal-newline, non-BMP, end-position, token-end, all negotiated LSP encodings, and MCP Unicode-scalar positions preserve the documented half-open selection rules. | Cross-adapter matrix with exact ranges and invalid-position/protocol-invalid cases. |
| Published reference generation | The generated catalog contains only checked language inputs and compiler-owned records; proposal text, maintenance routes, repository paths, and unpublished implementation prose are absent. Generation is deterministic and rejects stale or malformed inputs. | Generated artifact freshness, content-policy, input-schema, and deterministic digest checks. |
| Published reference search and read | Search applies the documented normalization, scopes, ranking, tie breaks, deduplication, scalar excerpts, and bounds. Standard resource reads and `read_doc` return identical complete bytes and metadata for every accepted topic URI; missing and generation-failure states do not publish partial content. | Search/ranking/bounds table, route byte-equality cases, URI rejection cases, and generation-failure state-preservation cases. |
| Published reference rendering | MCP and offline Markdown renderings preserve the catalog's topic identifiers, metadata, ordered semantic blocks, snippets, and expected-result text without introducing development-only content. | Cross-renderer semantic-model comparison and renderer-only stability cases. |
| Client plugins | Codex and Claude Code plugin manifests bind the active workspace, start the supported MCP contract, and isolate unknown files; the Claude route also completes its LSP lifecycle. Invalid manifests, unavailable servers, failed startup, and unknown-file inputs produce bounded client-visible failures without MCP stdout corruption. | Pinned native-client smoke cases, manifest/schema validation, startup-failure, MCP-failure, Claude-LSP-failure, and unknown-file isolation cases. |
| Skill routing | The shared agent skill searches and reads the published reference for language questions, routes implementation work to the repository authority, and reports unavailable or stale reference artifacts without inventing behavior. | Skill routing cases for known topics, unknown topics, stale catalogs, unavailable resources, and client-specific configuration. |
| Conformance gate | Every requirement has one stable identifier and passing evidence, every evidence route maps to a requirement, and stale or undeclared capabilities fail the gate. | Versioned conformance manifest tests for missing rows, duplicate IDs, orphaned evidence, missing matrix cells, stale artifacts, undeclared capabilities, malformed requests, and plugin mismatches. |

The remainder also includes broader definition navigation, unsupported package
symbol classes, transitive dependencies, recovery and casing-neutral lookup,
standard-library schema aliases, and any package-reference boundary not listed
as implemented above. These rows must retain explicit negative and
state-preservation cases: rejected requests do not mutate saved snapshots,
published resources, cursors, or prior successful results.

An implementation slice may be removed from this inventory only after its
current specification page names the executable authority, the evidence
passes independently of proposal prose, and the conformance manifest (when
present) has no missing or orphaned mapping. This preserves the umbrella as a
partial proposal rather than a second current-behavior specification.

## Non-goals

This proposal does not redefine the current `veln lsp` or `veln mcp` protocol,
the saved-workspace capture boundary, standard-library source resources, or
the implemented public standard-library schema composition and operation
references. Those contracts are owned by the current specification pages and
their checked adapter and language-service tests.
