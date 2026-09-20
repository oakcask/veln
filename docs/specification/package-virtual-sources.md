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

Noncanonical inputs include a non-lowercase scheme; any authority, user
information, host, port, query, or fragment; encoded ASCII unreserved bytes;
lowercase escape digits; a decoded package separator; an encoded source
separator; an empty or dot source segment; malformed escapes or UTF-8; and a
digest with any other length or spelling.

## References

The catalog implementation and URI boundary tests are in
`crates/veln-language-service/src/virtual_source.rs`.
