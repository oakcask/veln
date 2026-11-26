---
role: proposal
update-when: The proposed transport::net canonical adapter path, transport::network removal, consumer migration, or ownership-preservation acceptance changes.
---

# One Canonical Network Transport Adapter

Keep `std::transport::net` as the sole public network transport adapter.
Remove `std::transport::network` and its duplicate implementation. This is a
Ready proposal with no generic-function dependency.

## Current boundary and design choice

The [stdlib export manifest](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/veln.toml) exports
both modules. [transport/net.veln](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/transport/net.veln)
and [transport/network.veln](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/transport/network.veln)
both expose `read_stream_chunk`, `write_stream_chunks`, and `net_stream`.
They duplicate the same read validation and ordered write behavior rather
than expressing different transports or policies. HTTP/2 consumers use the
longer path, while direct companion coverage also uses the shorter path.

Choose `transport::net` because the owning public network module is named
`net` and the companion already checks that spelling. This is a naming and
ownership choice, not a claim that the longer word is generally wrong.
No inspected record establishes a need for two interchangeable public paths.

## Proposed contract

| Surface | Required behavior |
| --- | --- |
| Canonical import | `use transport::net from "std"` exposes the adapter. |
| Canonical operations | Preserve read, ordered chunk write, and DuplexStream handler semantics. |
| Removed import | External imports of `std::transport::network` fail resolution. |
| Export inventory and tooling | Only the canonical module is exported, documented, and offered. |

The canonical adapter retains its captured stream and nominal `net::IO`
effect. It does not silently install `net::system` or switch resources to a
new provider. Reads preserve bytes and clean EOF; an empty non-EOF chunk is
still rejected. Writes keep chunk order, stop on the first failure, and do not
send later chunks. An empty chunk list performs no network write. This
proposal does not add buffering, deadlines, retry, or progress accounting.

## Related work and alternatives

The inspected [Rust use-declaration reference](https://doc.rust-lang.org/reference/items/use-declarations.html#use-visibility)
shows that re-exports can create aliases without duplicate implementations.
That is a useful alternative when multiple paths have a consumer purpose.
Veln's project policy requires no backward compatibility here, and the two
inspected modules expose no distinct capability, so remove the redundant path
instead of preserving an alias. Rust's module syntax is not proposed for Veln.
The existing Veln manifest and both adapter bodies are the primary evidence
for this specific duplication.

## Planned acceptance and migration

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Canonical import and handler | Resolves externally and uses the captured resource. | Package lookup test and stdlib adapter companions |
| Old external import | Rejected even if a private distribution file accidentally remains. | Package export rejection test |
| Reads | Same bytes, EOF, failure, and empty-chunk validation. | Stdlib companions with scoped Veln support |
| Ordered writes and failure | No reordering or writes after the first failure. | Stdlib companions |
| HTTP/2 driver | Same bytes, outcomes, and ownership with canonical adapter. | Connection companions |
| Public bundle and docs | Duplicate module is absent and canonical operations remain. | Bundle tests, docs/navigation checks, symbol audit |

These checks are planned. Move reusable internal test support only where
needed; do not export support to retain the removed public path. Update the
HTTP/2 imports, shared adapter tests, bundle expectation, manifest, and
[networking specification](../specification/standard-library-networking.md)
together. No runtime adapter duplication or compatibility file remains after
migration. Historical references may remain only with an explicit historical
classification in the symbol audit.

There is no new syntax. Existing import formatting applies; ordinary formatting
does not rename imports. Explicit migration preserves aliases, effects, and
comments. Apply effect-handler maintenance when verifying captured provider
ownership. Publish current-behavior documentation and executable evidence when
implementation lands, then remove this proposal and its catalog entry.
