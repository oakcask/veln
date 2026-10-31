---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#published-boundary; limits=#generation-gates
update-when: The package documentation catalog API, canonical result bytes, digest, URI, gate, executable evidence, or veln doc command route changes.
---

# Package Documentation Catalogs

`veln-language-service` exposes a transport-independent package documentation
catalog for one validated package identity, one captured package snapshot, and
the validated manifest parsed from that same capture. Semantic publication
uses only the captured dependency projects supplied with that package and does
not reconstruct dependencies from the host filesystem. The package project
root is an analysis identity input and is not published in the catalog. The
catalog is not the `veln doc` Markdown output. `veln mcp` exposes a Markdown
projection for the embedded `std` package-documentation result and for
admitted direct-dependency package snapshots.

## Usage

Call `PackageDocResult::generate` with a validated `PackageIdentity`, a
`CapturedPackageSnapshot`, its `ProjectManifest`, the captured package project
root, every captured dependency project from the same analysis, and a
`PackageDocGeneratorContract`. Inspect `status()` for generation diagnostics
and `catalog()` for the optional published catalog. `canonical_bytes()` and
`doc_digest()` expose the stable result identity. Use `declaration_uri_for` or
`declaration_uri_for_location` to obtain documentation links rather than
constructing their identifiers.

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
status resources. Snapshot byte changes affect package snapshot identity and
the documentation digest because the canonical result contains that snapshot
digest. Catalog-semantic, schema, or generator-contract changes also affect
the documentation digest.

The implemented gates are parse, manifest, export, semantic visibility, module
value, documentation-reference, doctest, and identity.

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
snapshot segment. It also changes the `doc-digest` because the snapshot digest
is part of the canonical result bytes. A catalog-semantic, schema, or
generator-contract change changes the `doc-digest`. A renderer-only change
preserves the `doc-digest` only when it preserves the canonical result bytes.

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
signatures include effect row binders such as `<effect E>`. Effect declaration
signatures include the ordered operation names, parameter names and types, and
result types. Handler declaration signatures include the handler name and
parameters, the handled effect, and the retained effect row. Handler operation
clauses and bodies do not contribute to the signature or identity. A module
immutable value signature contains `let`, its public name, and its written type
annotation. Its initializer is not copied into declaration content and does not
contribute to the signature or declaration identity. Declaration identity does
not use source order or source byte offsets. A duplicate semantic identity
fails the complete package documentation result. A detected module or
declaration identifier collision also fails the complete package documentation
result. Initializer source bytes remain part of the package snapshot, so an
initializer-only source change rotates the snapshot identity, canonical result
bytes, and documentation digest.

## Published Boundary

The successful catalog includes only modules listed by `[lib].exports`. For
those modules it includes public type declarations, public type constructors,
public schemas, public effects and their operation signatures, public module
immutable values, public member aliases, public handlers, public functions,
attached documentation comments, public function contracts, visible doctest
fences, expected-output fences, and resolved schema documentation references.
Each public module value is a `module_value` declaration that publishes its
name and written type annotation without initializer text. Each public handler
is a distinct `handler` declaration with its canonical signature, stable
declaration identifier, and individually readable declaration resource. Public
type constructors carry their own attached documentation comments, visible
doctest fences, expected-output fences, and resolved schema documentation
references.
An effect is one declaration in the catalog; its operations are represented in
that declaration's canonical signature rather than as separate declarations.
Public function and ADT signatures preserve source-written variant-refinement
annotations, including refinements inside named arguments and function types.
A private refinement exposure in a public function signature or a public
variant payload of a public source ADT fails the semantic-visibility gate. The
package-atomic failure result contains no catalog, declaration URI, or
partially published signature. The gate still discovers a private refinement
in a function or named-type position when a sibling position has invalid
generic arity; that visibility failure rejects the complete catalog.
Publication dependency closure also rejects an exported public function or ADT
whose signature contains an ordinary named reference to a quarantined ADT,
including a reference through a transparent alias. The gate does not synthesize
a privacy diagnostic at each ordinary reference. If this closure makes any
manifest-exported declaration unavailable, retained causal privacy diagnostics
are eligible for the package-atomic failure even when their primary spans are
in non-exported sources.

The catalog excludes non-exported modules, private declarations, handler
operation clauses and bodies, module immutable value initializer text, exact
test companions, integration-test sources, hidden doctest setup lines, ADR-lite
records, raw manifests, dependency declarations, dependency selectors, local
paths, repository and homepage URLs, tool metadata, unknown manifest fields,
and environment-derived values. A direct successful `PackageDocResult` has a
declaration and lookup URI for each accepted public module value. A public
module value rejected by finite-data initializer, annotation, imported-lookup,
visibility, or package-wide dependency checking fails the package-atomic
result. No accepted sibling declaration is published in that failure result.
The `veln doc` command applies the same initializer, annotation, lookup,
visibility, and dependency rules. Its failure selection is broader: any
semantic error in the selected production project blocks the complete Markdown
document, except for the runtime-only `module.value_not_executable` diagnostic.

Published expected-output fences preserve the stream as `stdout` or `stderr`
and the complete lines for that stream.

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

Each retained diagnostic keeps the gate of its causal publication failure.
For example, one failed result can report a module-value diagnostic with
`gate: "module_value"` and an independent private-refinement diagnostic with
`gate: "semantic_visibility"`. The presence or ordering of another diagnostic
does not reclassify either failure. A retained cause from a captured dependency
keeps that dependency's package identity in its canonical `veln-pkg:` source
URI. A source path or byte range that is also present in the package being
documented does not reassign the cause to that package.

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
- semantic visibility: a public function signature or a public variant payload
  of a public source ADT must not expose a private variant-refinement base or
  selected variant through any structural type position. Semantic resolution
  uses every retained package source, including non-exported modules needed by
  imports and transparent aliases. A diagnostic rooted only in a retained
  non-exported declaration is non-blocking unless an exported public
  declaration depends on a quarantined ADT through its resolved signature.
  When dependency closure makes an exported declaration unavailable, only
  retained `type.variant_refinement_private` diagnostics in that declaration's
  causal dependency closure are admitted, even when their primary exposure
  spans are in non-exported sources. Related base, alias, and variant
  declarations may also be in a non-exported module. A failure uses
  `gate: "semantic_visibility"`, reports
  `type.variant_refinement_private`, and publishes no declaration identity or
  signature;
- module value: every public module immutable value in an exported source must
  pass finite-data initializer, written-annotation, imported-lookup,
  package-aware visibility, and package-wide dependency checking. The
  runtime-only `module.value_not_executable` diagnostic is not a documentation
  rejection. An error owned by that declaration or by a required module-value
  dependency rejects the complete catalog with `gate: "module_value"` and
  publishes no module value or accepted sibling declaration. The failure keeps
  only diagnostics in the rejected exported value's causal dependency closure.
  A public module value whose name does not start with an ASCII lowercase
  letter reports `name.invalid_case` through this gate. This name check applies
  to every retained source, including a source that is not exported;
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
URI. Effect declaration spans, effect-name tokens, operation declaration
spans, and operation-name tokens resolve to the owning effect declaration
documentation URI. Adapters return the URI from these lookups instead of
asking clients to construct resource identifiers or re-resolve by spelling.

## References

The public API is in
`crates/veln-language-service/src/package_documentation.rs`; catalog identity,
publication, and gate tests are in its `package_documentation/tests/` modules.
The `module_value_publication` tests check public declaration content and
lookup, initializer-independent declaration identity, snapshot and
documentation digest rotation, package-atomic failure, and causal diagnostic
selection for rejected values. They also check recovered-import causes and
dependency-owned causes whose paths or spans collide with the documented
package. Deterministic work counters compare adjacent input sizes for public
dependency chains, independent invalid values, and invalid-casing values to
guard linear semantic classification without a wall-time threshold.
The checked
[`standard-library-package-documentation-resources`](../../examples/specification/mcp/standard-library-package-documentation-resources/)
transcript covers the public `net::system()` handler resource, its canonical
signature and visible examples, the public `observe::noop()` and
`observe::recording(Recording)` handler resources, the production policy and
health types, the production constructor and health snapshot functions, the
`observe::production(Production)` handler resource, and the public `trace`
module and `trace::in_span` function resource. It also covers exclusion of
handler operation clauses, private host adapters, and private `Span`
constructors.
The [MCP specification](mcp.md) defines resource publication and links from
package-backed definition results.
