---
role: specification
authority: normative
review-when: The package documentation catalog API, canonical result bytes, digest, URI, gate, or executable-evidence contract changes.
---

# Package Documentation Catalogs

`veln-language-service` exposes a transport-independent package
documentation catalog for one `CapturedPackageSnapshot` and its validated
manifest. The result is immutable. It contains either a complete successful
catalog or a failure status with ordered diagnostics.

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
documentation comments, public function contracts, visible doctest fences, and
expected-output fences.

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
- export: every exported source must exist in the captured snapshot and each
  export path can appear at most once;
- documentation reference: `{@schema ...}` references in exported source
  documentation must resolve to a public schema in an exported module;
- doctest: visible `veln` doctest fences must parse unless they declare an
  expected error, expected-error doctests must produce a parse diagnostic whose
  id matches the `error=` value, and unknown doctest fence attributes fail
  generation; and
- identity: duplicate semantic declaration identities fail generation.
  Detected module identifier collisions and declaration identifier collisions
  fail generation.

Failure diagnostics are sorted by source URI, start range, diagnostic code,
and message.

## Declaration Lookup

The catalog provides a transport-independent lookup from module name,
declaration kind, and declaration name to the canonical declaration
documentation URI in the same snapshot. Adapters return the URI from this
lookup instead of asking clients to construct resource identifiers.

## Executable Evidence

The `veln-language-service` package-documentation unit tests are the
authoritative executable evidence. `cargo test -p veln-language-service`
checks public metadata allowlisting, exported-module and public-declaration
selection, private and non-exported exclusion, contracts, constructor
projection, visible doctest and expected-output publication, hidden setup and
ADR-lite exclusion, deterministic bytes, digest and URI stability, package
byte changes, generator-contract changes, renderer-only stability when bytes
are unchanged, declaration URI lookup, parse gate failure, export gate
failure, documentation-reference gate failure, doctest gate failure, duplicate
semantic identity failure, declaration identifier collision failure,
successful negative doctest publication, expected negative-doctest diagnostic
matching, status-only failure results, and virtual-source resolution after
package documentation failure.

The readable CLI documentation boundary remains checked by
`examples/specification/doc/`. The transport-independent catalog itself is a
Rust API and is not exposed by `veln doc` or MCP in this slice.
