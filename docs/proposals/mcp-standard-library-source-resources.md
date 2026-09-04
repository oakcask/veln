---
role: proposal
update-when: The embedded standard-library bundle, package virtual-source catalog, MCP resource contract, or planned standard-library source-resource evidence changes.
---

# MCP Standard Library Source Resources

## Summary

Publish every distribution source in the embedded Veln standard library as an
immutable MCP resource. This slice gives clients an exact read route for
canonical `veln-pkg:` source URIs without adding dependency loading, package
documentation, navigation expansion, or mutable snapshot state.

## Scope

| Included | Excluded |
| --- | --- |
| One validated embedded `std` package snapshot created from the shipped standard-library bundle. | Workspace and dependency package snapshots. |
| Canonical `veln-pkg:` metadata for every standard-library distribution source. | Package-documentation and language-reference search expansion. |
| Exact source reads through the existing MCP resource routes. | Definition and reference results for standard-library symbols. |
| Deterministic ordering, exact URI lookup, and lifecycle preservation. | Resource templates, pagination, subscriptions, and snapshot eviction. |
| Unit, protocol, and stdio acceptance evidence. | LSP changes, client plugins, and conformance-manifest completion. |

The shared package-snapshot capture and virtual-source catalog are the primary
authorities for snapshot membership, digesting, URI construction, canonical
spelling, and exact-byte resolution. This proposal defines only their bounded
MCP publication for the repository-owned embedded standard library.

## Snapshot And Resource Contract

Server startup creates one embedded package snapshot with the reserved `std`
identity from the manifest and distribution sources in the shipped
`veln-stdlib` bundle. The capture uses the shared embedded-package validation
and the same test-source exclusions as the bundle and package-snapshot
contracts. Failure to validate the embedded bundle or construct its
virtual-source catalog fails server startup; the server does not publish a
partial standard-library set.

`resources/list` includes exactly one metadata entry for each virtual-source
catalog entry in the captured `std` snapshot. Standard-library source metadata
uses the canonical `veln-pkg:` URI, the package-relative source path as its
name, a title formed as `Veln standard library source: {path}`, and media type
`text/x-veln; charset=utf-8`. The existing language-reference resources remain
present. The combined resource list sorts by URI UTF-8 bytes and contains no
duplicate URI. It remains one complete response without `nextCursor`.

`resources/read` accepts each listed standard-library source URI and returns
one complete text content entry. Its URI and media type equal the listed
metadata, and its text contains the exact UTF-8 source bytes retained by the
captured snapshot. The server does not read the standard-library source tree or
another materialization path after startup.

Lookup uses exact catalog membership. An unknown identity, digest, or source
path and every malformed or noncanonical `veln-pkg:` spelling return the MCP
resource-not-found protocol error with structured domain code
`resource_not_found`. The server does not decode, normalize, rewrite, or fall
back to the filesystem for a rejected URI. Existing malformed
`resources/list` and `resources/read` parameter behavior does not change.

## Lifecycle Boundary

The embedded snapshot, source resource set, metadata, URIs, and bytes are fixed
for the server lifetime. Workspace discovery, a successful or failed
`refresh_workspace`, project analysis, and language-reference search or reads
do not change them. Invalid requests and missing-resource failures preserve all
workspace and resource state.

This slice admits only the single embedded snapshot, so it does not introduce
the package-snapshot capacity, coexistence, or eviction behavior required for
loaded dependencies. A later dependency-resource proposal must define those
state transitions independently.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Start the server with the shipped standard-library bundle. | One validated embedded `std` snapshot supplies the complete virtual-source catalog; invalid bundle or catalog input fails startup without partial resources. | Embedded-capture and startup-failure unit cases. |
| List resources after initialization. | The combined list contains every language resource and exactly one entry for every standard-library distribution source, sorted by URI bytes with no cursor or duplicate. | Catalog-to-resource bidirectional completeness test and exact stdio list case. |
| Read each listed standard-library source. | URI and media type equal the listed metadata, and text equals the captured bundle bytes exactly, including line endings. | Table-driven route round trips and representative exact stdio reads. |
| Read private and non-exported distribution sources. | They remain readable because distribution membership, not semantic visibility, controls source publication. | Exported and private source read cases. |
| Read a test-shaped source or a path absent from the captured distribution. | No such resource is listed, and exact reads return `resource_not_found`. | Distribution-exclusion and missing-path cases. |
| Read an unknown-identity, wrong-digest, malformed, or noncanonical `veln-pkg:` URI. | The request returns structured `resource_not_found` without normalization or filesystem fallback. | Shared virtual-source rejection table mapped through MCP. |
| Refresh the workspace, analyze a project, and use the language-reference tools between list and read requests. | Standard-library resource metadata, URIs, ordering, and bytes remain identical. | MCP lifecycle state-preservation case. |
| Supply malformed list or read parameters. | Existing invalid-params behavior remains unchanged and publishes no partial resource content. | Protocol parameter matrix. |

## Completion

This proposal is complete when every acceptance row passes and the MCP
specification and executable examples state the implemented standard-library
source-resource behavior. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
