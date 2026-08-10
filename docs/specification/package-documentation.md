---
role: specification
authority: normative
update-when: The package documentation catalog API, canonical result bytes, digest, URI, gate, executable evidence, or detail-page route changes.
---

# Package Documentation Catalogs

`veln-language-service` exposes a transport-independent package documentation
catalog for one validated package identity, one captured package snapshot, and
the validated manifest parsed from that same capture. The catalog is not the
`veln doc` Markdown output and is not exposed through MCP in the current slice.

## Read First

- Use [package-documentation-full.md](package-documentation-full.md) when
  changing catalog identity, published fields, gates, declaration lookup,
  canonical bytes, documentation digests, or `veln-doc:` URI construction.
- Use [package-snapshots.md](package-snapshots.md) when changing captured
  package snapshot inputs, distribution-source retention, package snapshot
  digests, or embedded snapshot capture.
- Use [package-virtual-sources.md](package-virtual-sources.md) when changing
  `veln-pkg:` URI listing or exact virtual-source resolution.
- Use [commands.md](commands.md) and [commands-full.md](commands-full.md) when
  changing `veln doc` Markdown generation, schema-reference diagnostics, or
  doctest output comparison for CLI commands.

## Current Contract

The package documentation catalog is package-atomic. Generation publishes a
complete immutable catalog or a status-only failure result with ordered
diagnostics. The catalog binds to the same-capture manifest bytes, exports only
the public API of manifest-listed modules, publishes a closed package metadata
allowlist, and excludes private implementation details, raw manifests,
dependencies, local paths, URLs, tool metadata, unknown manifest fields, and
environment-derived values.

Catalog identity uses canonical result bytes and a `doc-digest`. Successful
resources use stable `veln-doc:` URIs for index, module, declaration, and
status resources. Snapshot byte changes affect package snapshot identity.
Catalog-semantic, schema, or generator-contract changes affect the
documentation digest.

The implemented gates are parse, manifest, export, documentation-reference,
doctest, and identity. The authoritative executable evidence is
`cargo test -p veln-language-service` plus package catalog fixtures under
`examples/specification/doc/`.
