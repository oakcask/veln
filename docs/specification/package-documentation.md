---
role: specification
authority: normative
review-when: The package documentation catalog API, canonical result bytes, digest, URI, gate, or executable-evidence contract changes.
---

# Package Documentation Catalogs

`veln-language-service` exposes a transport-independent package
documentation catalog for one `CapturedPackageSnapshot` and the validated
manifest parsed from that same capture. The generator validates the manifest
before it can publish a catalog. The result is immutable. It contains either a
complete successful catalog or a failure status with ordered diagnostics.

## Result Identity

The catalog result uses schema version `veln-package-doc-catalog/v1` and a
caller-supplied generator-contract version. The `doc-digest` is SHA-256 over
this transcript:

```text
ASCII "veln-package-doc-catalog/v1\0"
u64be(canonical result byte length)
canonical result bytes
```

The digest has exactly 64 lowercase hexadecimal digits. A package snapshot
byte change changes the package snapshot digest and the documentation URI
snapshot segment. A catalog-semantic, schema, or generator-contract change
changes the `doc-digest`. A renderer-only change preserves the `doc-digest`
only when it preserves the canonical result bytes.

Canonical result bytes contain the catalog semantics or failure status. They
do not contain the resource URIs that are derived from the `doc-digest`.

Successful resources use these canonical URI forms:

```text
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/index
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/module/<module-id>
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/declaration/<declaration-id>
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/status
```

The package segment uses the same segment encoding as package virtual-source
URIs. Module and declaration identifiers are 64-character lowercase SHA-256
digests from versioned identity domains. Declaration identity uses declaration
kind, fully qualified semantic name, and canonical signature. It does not use
source order or source byte offsets. A duplicate semantic identity fails the
complete package documentation result. A detected module or declaration
identifier collision also fails the complete package documentation result.

## Published Boundary

The successful catalog includes only modules listed by `[lib].exports`. For
those modules it includes public type declarations, public type constructors,
public schemas, public member aliases, public functions, attached
documentation comments, public function contracts, visible doctest fences,
expected-output fences, and resolved schema documentation references.

The catalog excludes non-exported modules, private declarations, exact test
companions, integration-test sources, hidden doctest setup lines, ADR-lite
records, raw manifests, dependency declarations, dependency selectors, local
paths, repository and homepage URLs, tool metadata, unknown manifest fields,
and environment-derived values.

Published package metadata is limited to package identity, manifest package
name, version, description, license, authors, keywords, and exported module
names.

## Generation Gates

Generation is package-atomic. If any gate fails, the result contains only a
status object and no module or declaration catalog. The source snapshot and
virtual-source catalog remain usable by their own APIs.

The implemented gates are:

- parse: all retained package sources must parse;
- manifest: unsupported manifest sections, invalid export paths, test
  companion exports, duplicate exported module identities, invalid direct
  git selector cardinality, and validated manifest bytes that differ from the
  captured snapshot manifest bytes fail generation before a catalog is
  published;
- export: every exported source must exist in the captured snapshot and each
  export path can appear at most once;
- documentation reference: `{@schema ...}` references in documentation
  blocks that attach to exported public declarations must resolve to a public
  schema in an exported module, and the successful catalog keeps the resolved
  target declaration identifier and same-snapshot declaration URI;
- doctest: visible positive `veln` doctest fences must pass the shared
  generated-source static analysis pipeline, `veln fail` fences must produce
  an error diagnostic through that same pipeline, `veln ignore` fences are not
  published or checked by the catalog, hidden setup lines are not published,
  `veln-output stream=stdout` and `veln-output stream=stderr` fences are
  accepted, and doctest metadata diagnostics from the shared doctest extractor
  fail generation; and
- identity: duplicate semantic declaration identities fail generation.
  Detected module identifier collisions and declaration identifier collisions
  fail generation.

Failure diagnostics are sorted by source URI, start range, diagnostic code,
and message.

## Declaration Lookup

The catalog provides transport-independent lookup from module name,
declaration kind, and declaration name to the canonical declaration
documentation URI in the same snapshot. It also accepts a snapshot-bound
semantic `NavigationLocation` for public declarations. Declaration-span and
name-token locations resolve to the same declaration documentation URI.
Constructor declaration-span and name-token locations resolve to the owning
type declaration documentation URI. Adapters return the URI from these lookups
instead of asking clients to construct resource identifiers or re-resolve by
spelling.

## Executable Evidence

The `veln-language-service` package-documentation unit tests are the
authoritative executable evidence. `cargo test -p veln-language-service`
checks public metadata allowlisting, exported-module and public-declaration
selection, private and non-exported exclusion, contracts, constructor
projection and constructor-location lookup, visible doctest and expected-output
publication, generated doctest static analysis, `veln fail` doctest handling,
hidden setup and ADR-lite exclusion, deterministic bytes, digest and URI
stability, package byte changes, generator-contract changes, renderer-only
stability when bytes are unchanged, declaration URI lookup from declaration
spans and navigation name-token spans, private documentation-reference
exclusion, parse gate failure with canonical package source URI, manifest gate
failure, manifest snapshot-byte mismatch failure, export gate failure,
resolved documentation-reference projection, documentation-reference gate
failure, doctest gate failure, duplicate semantic identity failure,
declaration identifier collision failure, status-only failure results, and
virtual-source resolution after package documentation failure. The tests also
read fixtures under `examples/specification/doc/` to observe the catalog
success path, manifest-gate failure path, and doctest static-gate failure path
through executable specification inputs.

The readable CLI documentation boundary remains checked by
`examples/specification/doc/`. The transport-independent catalog itself is a
Rust API and is not exposed by `veln doc` or MCP in this slice.
