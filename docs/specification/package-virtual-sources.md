---
role: specification
authority: normative
update-when: The virtual-source URI, catalog, resolver, or executable-evidence contract changes.
---

# Package Virtual Sources

`veln-language-service` exposes a transport-independent virtual-source catalog
for validated `PackageIdentity` values and immutable
`CapturedPackageSnapshot` values. The catalog contains exactly one entry for
each retained distribution source. Duplicate canonical entries cause catalog
construction to fail.

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

## Executable Evidence

The `veln-language-service` virtual-source unit tests are the authoritative
executable evidence. `cargo test -p veln-language-service` checks canonical
round trips, identity and source-segment encoding, Unicode, relocation
independence, digest changes, exact-byte reads, identity/digest/path
mismatches, every rejection class above, duplicate entries, and equality
between the captured distribution set and the listable and resolvable set.

A Veln source example is not present under `examples/specification/` because
the catalog is a transport-independent Rust API. It does not add Veln source
syntax or command behavior.
