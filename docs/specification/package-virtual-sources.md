---
role: specification
authority: normative
specification-coverage: usage=#package-virtual-sources; behavior=#package-virtual-sources; limits=#package-virtual-sources
update-when: The virtual-source URI, catalog, resolver, or executable-evidence contract changes.
---

# Package Virtual Sources

`veln-language-service` exposes a transport-independent virtual-source catalog
for validated `PackageIdentity` values and immutable
`CapturedPackageSnapshot` values. The catalog contains exactly one entry for
each retained distribution source. Duplicate canonical entries cause catalog
construction to fail.

Construct `VirtualSourceCatalog::new` with `(PackageIdentity,
CapturedPackageSnapshot)` pairs. Use `entries()` to list canonical URIs and
`resolve(uri)` to retrieve retained bytes. `entry_for_source(package_index,
source_index)` returns the entry at the supplied capture indices, or `None`
when either index is out of range. Listing preserves the supplied package
order and each snapshot's source order.

Each entry has this canonical URI:

```text
veln-pkg:///<package-segment>/snapshot/<digest>/<source-path>
```

The package identity is one URI path segment. Each source-path segment is
encoded separately. ASCII letters, digits, `-`, `.`, `_`, and `~` remain
literal. Every other UTF-8 byte uses percent encoding with uppercase
hexadecimal digits. The digest is exactly the snapshot's 64 lowercase
hexadecimal digits.

Resolution accepts only a URI string already present in the catalog. It
returns the exact bytes retained in the captured snapshot. It does not parse,
decode, normalize, or rewrite the input. It does not consult a materialization
path or the filesystem. Therefore an unknown identity, digest, or source path
has the same not-found result as a malformed or noncanonical URI.

When two project snapshots retain different standard-library source bytes,
each snapshot resolves only the canonical `std` URI for its own retained
revision. The other revision's URI has the not-found result. A direct
dependency whose identity and source bytes are unchanged keeps the same URI
and remains resolvable in both snapshots.

The LSP resolver retains one URI-to-bytes entry for each distinct dependency
virtual source exposed by a replaced layer and not available from the current
layer. It does not retain the replaced prepared analysis snapshot. Navigation
after a successfully accepted dependency payload change uses the new snapshot
URI. A previously returned snapshot URI remains resolvable to its original
bytes while that LSP project-snapshot state is retained. If a later payload
restores an earlier snapshot URI, the current layer supplies that URI and the
resolver removes its redundant retired entry. Repeated transitions between the
same payloads therefore do not increase retained virtual-source storage or make
lookup depend on the number of transitions.

A named-generic or function-variance publication rejection does not replace the
current dependency layer and does not create a retired resolver entry. The
current accepted snapshot URI remains resolvable. A URI that exists only in the
rejected capture is not resolvable. If no dependency layer has succeeded, the
first rejected capture publishes no virtual-source URI or bytes. A later
accepted capture replaces the layer through the ordinary transition above.
The [LSP publication gate](editor-support.md#lsp-navigation-formatting-and-rename)
defines which diagnostics reject that transition.

Noncanonical inputs include a non-lowercase scheme; any authority, user
information, host, port, query, or fragment; encoded ASCII unreserved bytes;
lowercase escape digits; a decoded package separator; an encoded source
separator; an empty or dot source segment; malformed escapes or UTF-8; and a
digest with any other length or spelling.

## References

The catalog implementation and URI boundary tests are in
`crates/veln-language-service/src/virtual_source.rs`.
The project-snapshot standard-library revision and unchanged-owner regression
is in
`crates/veln-language-service/src/tests/navigation_handler_references.rs`.
The dependency-payload revision, coupled diagnostic and navigation capture,
repeated-revision storage, and earlier-URI retention regression is in
`crates/veln-lsp/src/tests/dependencies.rs`.
