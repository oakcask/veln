---
role: specification
authority: normative
update-when: The package documentation catalog API, canonical result bytes, digest, URI, gate, executable evidence, or veln doc command route changes.
---

# Package Documentation Catalogs

`veln-language-service` exposes a transport-independent package documentation
catalog for one validated package identity, one captured package snapshot, and
the validated manifest parsed from that same capture. The catalog is not the
`veln doc` Markdown output. `veln mcp` currently exposes a Markdown projection
for the embedded `std` package-documentation result; direct-dependency
documentation resources remain planned.

## Read First

- Use [package-snapshots.md](package-snapshots.md) when changing captured
  package snapshot inputs, distribution-source retention, package snapshot
  digests, or embedded snapshot capture.
- Use [package-virtual-sources.md](package-virtual-sources.md) when changing
  `veln-pkg:` URI listing or exact virtual-source resolution.
- Use [command-doc.md](command-doc.md) when changing `veln doc` Markdown
  generation, schema-reference diagnostics, or doctest output comparison for
  CLI commands.
- Use [mcp.md](mcp.md) when changing MCP publication, resource templates,
  exact resource reads, or resource-not-found behavior for rendered package
  documentation.

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
`examples/specification/doc/`. The generated Markdown fixtures keep doctest
fences as ordered evidence between inline expected-output fragments, so
fixture-manifest fragment placement can change without changing the
documentation catalog contract.

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
digests from versioned identity domains. Each identifier transcript encodes
every part length as a checked unsigned 64-bit big-endian integer before the
part bytes, so the identifier is independent of target pointer width. Module
identity is derived from the package-relative source path. It is not derived
from a `module` declaration. Declaration identity uses declaration kind, fully
qualified semantic name, and canonical signature. Function declaration
signatures include effect row binders such as `<effect E>`. Declaration
identity does not use source order or source byte offsets. A duplicate
semantic identity fails the complete package documentation result. A detected
module or declaration identifier collision also fails the complete package
documentation result.

## Published Boundary

The successful catalog includes only modules listed by `[lib].exports`. For
those modules it includes public type declarations, public type constructors,
public schemas, public member aliases, public functions, attached
documentation comments, public function contracts, visible doctest fences,
expected-output fences, and resolved schema documentation references. Public
type constructors carry their own attached documentation comments, visible
doctest fences, expected-output fences, and resolved schema documentation
references.

The catalog excludes non-exported modules, private declarations, exact test
companions, integration-test sources, hidden doctest setup lines, ADR-lite
records, raw manifests, dependency declarations, dependency selectors, local
paths, repository and homepage URLs, tool metadata, unknown manifest fields,
and environment-derived values. Published expected-output fences preserve the
stream as `stdout` or `stderr` and the complete lines for that stream.

Published package metadata is limited to package identity, manifest package
name, version, description, license, authors, keywords, and exported module
names. Exported module names are derived from validated, normalized export
paths. A normalized export path for a `main.veln` source publishes module
`main`. An exported source path must derive a valid source module path under
the same source-path rules used by compiler analysis, including the
source-path identifier casing boundary specified by
[name-resolution.md](name-resolution.md).
If an exported source path has a source-path casing diagnostic, generation
returns a status-only failure result. The result publishes no catalog,
module, declaration, exported-module metadata, or declaration lookup result,
even when other exported sources in the same package have valid lowercase
paths. If that invalid-cased export path is absent from the captured snapshot,
generation returns the existing `package_doc.missing_export` status-only
failure instead of dropping the manifest-listed export from validation.

## Generation Gates

Generation is package-atomic. If any gate fails, the result contains only a
status object and no module or declaration catalog. The source snapshot and
virtual-source catalog remain usable by their own APIs.

The implemented gates are:

- parse: all retained package sources must parse;
- manifest: a missing package name, a package name that differs from the
  supplied package identity, unsupported manifest sections, invalid export
  paths, test source exports, duplicate exported module identities, invalid
  direct git selector cardinality, and validated manifest bytes that differ
  from the captured snapshot manifest bytes fail generation before a catalog
  is published;
- export: every exported source must exist in the captured snapshot and each
  export path can appear at most once;
- documentation reference: `{@schema ...}` references in documentation
  blocks that attach to exported public declarations or exported public type
  constructors must resolve to a public schema in an exported module. Bare
  references resolve in the same module. Qualified references require a
  matching written package-local `use` path in the referencing module. Public
  schema aliases resolve to their public schema target. The successful catalog
  keeps the resolved target declaration identifier and same-snapshot
  declaration URI;
- doctest: only visible doctest fences attached to exported modules, exported
  public declarations, and exported public type constructors are gate inputs.
  If a continuous documentation block is classified as ADR-lite by its first
  non-empty documentation line, all fenced examples in that block are excluded
  from package documentation doctest gates. Visible positive `veln` doctest
  fences must pass the shared generated-source static analysis pipeline,
  including the declaration and statement portions of visible positive
  doctests that contain both. Declaration doctests can contain nested
  expression blocks and public member aliases; declaration spans from the
  parsed doctest source determine which visible lines are declarations before
  later statement examples are checked. Positive declaration doctests can call
  public API from the same exported package snapshot. `veln fail` fences must
  produce a parse diagnostic through that same pipeline. A semantic-only
  diagnostic does not satisfy `veln fail`. Static-gate diagnostics that
  originate in generated doctest sources report the canonical `veln-pkg:` URI
  and line, column, and offset for the originating visible doc comment line in
  the same captured source. `veln ignore` fences are not published or checked
  by the catalog, hidden setup lines are not published or checked by the
  catalog, `veln-output stream=stdout` and `veln-output stream=stderr` fences
  are accepted, duplicate output fences for the same stream fail generation,
  and expected-output fences attach only while the same pending-state boundary
  used by the shared doctest extractor remains open. The catalog publishes
  visible doctests from the same shared doctest extraction result that
  supplies doctest gate diagnostics. Duplicate or ambiguous expected-output
  stream metadata fails generation with the shared doctest metadata
  diagnostic; and
- identity: duplicate semantic declaration identities fail generation.
  Detected module identifier collisions and declaration identifier collisions
  fail generation.

Failure diagnostics are sorted by source URI, start range, diagnostic code,
and message.

## Declaration Lookup

The catalog provides transport-independent lookup from module name,
declaration kind, and declaration name to the canonical declaration
documentation URI in the same snapshot. It also accepts a snapshot-bound
semantic `NavigationLocation` for public declarations only when the location
uses `NavigationSource::Package` with a canonical `veln-pkg:` URI from the
same package identity and snapshot digest. `NavigationSource::Workspace`
locations and package URIs from another snapshot do not resolve to package
documentation URIs. Declaration-span and name-token locations resolve to the
same declaration documentation URI. Constructor declaration-span and
name-token locations resolve to the owning type declaration documentation
URI. Adapters return the URI from these lookups instead of asking clients to
construct resource identifiers or re-resolve by spelling.

## Executable Evidence

The `veln-language-service` package-documentation unit tests are the
authoritative executable evidence. `cargo test -p veln-language-service`
checks:

- catalog selection and projection;
- doctest analysis and metadata;
- identity and deterministic output;
- declaration and package navigation location lookup;
- generation gates and status-only failure results;
- public package documentation API imports.

The tests also read fixtures under `examples/specification/doc/` to observe:

- the catalog success path;
- the manifest-gate failure path;
- the nested declaration doctest success path with a nested expression block
  and public member alias;
- ADR-lite doctest exclusion from successful catalog generation;
- the declaration doctest static-gate failure path;
- the doctest output metadata gate failure path;
- integration-test source exclusion from successful catalog projection;
- the schema-reference import-gate failure path through executable
  specification inputs;
- the source-path casing failure path for an exported source that has a valid
  exported sibling, and the missing-export path for an absent invalid-cased
  export beside a valid sibling, proving the status-only package-atomic
  boundary.

The readable CLI documentation boundary remains checked by
`examples/specification/doc/`. The transport-independent catalog itself is a
Rust API and is not exposed by `veln doc`. MCP exposure for the embedded
standard-library documentation result is specified in [mcp.md](mcp.md) and
checked by `examples/specification/mcp/standard-library-package-documentation-resources/`.
