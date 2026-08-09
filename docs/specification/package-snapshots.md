---
role: specification
authority: normative
review-when: The package snapshot capture API, distribution-set rules, digest transcript, or executable evidence changes.
---

# Package Snapshot Digests

`veln-project` captures a package distribution from a filesystem root and
computes a package snapshot digest from that immutable capture. Callers that
already own exact bytes can use the digest API directly. Duplicate source
paths supplied to the digest API are rejected.

## Filesystem Capture

`capture_package_snapshot` requires a regular `veln.toml` at the supplied
package root. The returned capture retains the manifest's exact bytes, the
ordered owned sources and their exact bytes, and the digest computed from that
same retained data.

An owned source is a regular file whose package-relative path ends in `.veln`.
The capture includes private, non-exported, on-disk generated, and ordinary
`target` sources. It excludes these entries:

- each `.git` directory and its descendants;
- each descendant directory that contains a regular `veln.toml` and all of
  that directory's descendants;
- every symbolic link;
- paths that end in `.test.veln`; and
- paths that end in `_test.veln`.

Source paths use UTF-8 and `/` separators. They are ordered by their UTF-8
bytes. A discovered path that has no exact UTF-8 representation causes an
explicit capture error. The capture does not perform lossy path or source-byte
conversion.

The digest is independent of filesystem enumeration order and the package's
physical parent location. A change to the manifest bytes, an included source
path, or included source bytes changes the digest.

## Digest Transcript

SHA-256 consumes this exact compatibility transcript:

```text
ASCII "veln-package-snapshot/v1\0"
0x01 || u64be(manifest byte length) || exact manifest bytes
0x02 || u64be(source count)
for each source sorted by normalized path UTF-8 bytes:
  0x03 || u64be(path byte length) || path UTF-8 bytes
       || u64be(source byte length) || exact source bytes
```

All lengths and the source count are unsigned 64-bit big-endian integers. The
transcript has no terminal record. The result is exactly 64 lowercase
hexadecimal digits without a prefix.

The package snapshot digest is separate from the lockfile source-tree
checksum. The lockfile checksum retains its `sha256:` prefix and its existing
transcript.

## Executable Evidence

The `veln-project` snapshot unit tests are the authoritative executable
evidence. `cargo test -p veln-project` checks the capture distribution matrix,
exact-byte digest integration, deterministic path ordering, relocation,
descendant package boundaries, symbolic links, non-UTF-8 path rejection, and
these fixed digest vectors:

| Manifest and sources | Digest |
| --- | --- |
| Empty manifest; no sources | `f0030b92642915b495c426a5b5185676e0306219a52c448a94fb5e8dccc494ad` |
| Manifest `[package]\nname = "p"\n`; `a.veln` is `a\n`; `z.veln` is `z\n` | `77150b975c9bb56aab9e9b3c8899a81907abc9db535fdfbb6276d40bff9fa878` |
| Empty manifest; `src/λ.veln` is `λ\n` | `f360e18455f6b7c90dd6c34cdec7a444082e003e44583dc8a7d99ae50cba713b` |

The same tests check reversed source order, duplicate paths, and isolated
changes to the domain, record tags, byte order, manifest bytes, source path
bytes, and source content bytes. A Veln source example is not added because
this contract is a transport-independent Rust API and has no Veln source or
command behavior.
