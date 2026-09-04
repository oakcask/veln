---
role: proposal
update-when: Direct-dependency capture, package virtual-source publication, MCP resource lifecycle, or planned dependency source-resource evidence changes.
---

# MCP Dependency Source Resources

## Summary

Publish the exact distribution sources of direct dependencies captured by a
successful saved-project operation through the existing MCP resource routes.
This slice extends the immutable standard-library source-resource contract to
validated dependency snapshots without adding package documentation,
dependency navigation, or resource-list pagination.

## Scope

| Included | Excluded |
| --- | --- |
| Validated direct-dependency snapshots captured for selected manifest projects. | Workspace-project sources and unresolved dependency declarations. |
| Atomic admission, digest-based deduplication, identity coexistence, and a fixed server-lifetime capacity. | Transitive dependency discovery and snapshot eviction. |
| Canonical `veln-pkg:` metadata and exact source reads for every admitted distribution source. | Package documentation, documentation search, and navigation expansion. |
| Deterministic combined listing and exact URI lookup through existing MCP routes. | Resource templates, pagination, subscriptions, and list-change notifications. |
| Unit, protocol, and stdio acceptance evidence. | LSP changes, client plugins, and conformance-manifest completion. |

The package-snapshot and virtual-source specifications remain authoritative for
distribution membership, validation, digesting, URI spelling, and exact-byte
resolution. The MCP specification owns the already implemented language and
standard-library resource behavior. This proposal defines only dependency
snapshot admission and publication through that resource surface.

## Admission Contract

A successful `check_project`, `definition`, or `references` operation on a
selected manifest project considers every available direct dependency captured
from the same stable saved-project snapshot. Each dependency must have a valid
manifest package identity that equals the dependency table key and a valid
captured package snapshot. An unavailable or invalid dependency remains an
analysis input failure or diagnostic under the owning operation contract; it
does not create a resource entry.

The server admits a dependency snapshot only after the owning operation has
completed its stable capture and validation. Admission is atomic for that
operation. A transport error, domain failure, capture retry exhaustion, or
validation failure publishes no new snapshot.

Snapshots are keyed by exact package identity and digest. Repeating an admitted
pair adds no state. Different digests for one identity coexist and retain their
distinct URIs. The server admits at most 256 package snapshots, including the
embedded standard-library snapshot. If an operation would exceed the remaining
capacity, it returns `resource_capacity`, publishes none of that operation's
new snapshots, and preserves every previously admitted snapshot.

Every admitted snapshot remains available until server shutdown. Workspace
refresh, project removal, dependency removal, and a later digest for the same
identity do not remove or replace it.

## Resource Contract

`resources/list` includes exactly one metadata entry for each distribution
source in every admitted dependency snapshot. Dependency source metadata uses
the canonical `veln-pkg:` URI, the package-relative source path as `name`, a
title formed as `Veln package source: {identity}: {path}`, and media type
`text/x-veln; charset=utf-8`. Existing language-reference and standard-library
resources remain present.

The combined resource list is sorted by URI UTF-8 bytes, contains no duplicate
URI, and remains one complete response without `nextCursor`. Distribution
membership controls publication, so private and non-exported sources are
listed while excluded test sources are not.

`resources/read` accepts every listed dependency source URI and returns one
complete text content entry. Its URI and media type equal the listed metadata.
Its text equals the exact UTF-8 bytes retained by the admitted snapshot. Reads
use retained bytes and do not consult the dependency materialization path.

Lookup requires exact catalog membership. Unknown, no-longer-current,
malformed, and noncanonical `veln-pkg:` URIs return the existing
resource-not-found protocol error with structured domain code
`resource_not_found`. The server does not normalize a rejected URI or fall
back to the filesystem.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Complete a saved-project operation with one valid direct dependency. | The response succeeds and atomically admits the dependency's identity and captured digest. | Table-driven operation and admission unit cases. |
| Complete an operation with repeated dependencies or repeat the same capture. | Each identity-and-digest pair is admitted once and the resource set contains no duplicate URI. | Deduplication cases across operations and selected projects. |
| Capture a new digest for an already admitted identity. | Both snapshots remain readable through distinct canonical URIs until shutdown. | Same-identity coexistence and exact-byte read case. |
| Fail capture, validation, or the owning tool operation. | No new snapshot or resource is published; prior resources remain byte-identical. | Injected failure and state-preservation cases. |
| Admit through the remaining capacity and then exceed it with one or several new snapshots. | The boundary admission succeeds; the exceeding operation returns `resource_capacity` and admits none of its new snapshots. | Boundary and atomic multi-snapshot capacity cases. |
| List resources after dependency admission. | Existing resources and exactly one entry per admitted distribution source appear in URI-byte order with no cursor. | Catalog-to-list completeness unit test and stdio lifecycle case. |
| Read exported, private, and non-exported dependency sources. | Every retained distribution source returns its exact captured bytes and metadata. | Table-driven route round trips and representative stdio reads. |
| Read an excluded test source or unknown, stale-looking, malformed, or noncanonical URI. | The request returns `resource_not_found` without normalization or filesystem access. | Distribution exclusion and shared virtual-source rejection matrix mapped through MCP. |
| Refresh the workspace or remove, relocate, or modify a dependency after admission. | Every admitted URI and byte sequence remains readable; a later valid digest can coexist but cannot mutate the prior snapshot. | MCP lifecycle transition cases using retained captures. |
| Supply malformed list or read parameters. | Existing invalid-params behavior remains unchanged and publishes no partial state. | Protocol parameter matrix. |

## Completion

This proposal is complete when every acceptance row passes and the MCP
specification and executable examples state the implemented dependency
source-resource behavior. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
