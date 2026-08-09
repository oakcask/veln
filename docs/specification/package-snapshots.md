---
role: specification
authority: normative
review-when: The package snapshot digest API, transcript version, or fixed-vector evidence changes.
---

# Package Snapshot Digests

`veln-project` computes a package snapshot digest from exact manifest bytes and
a unique set of normalized UTF-8 source paths paired with exact source bytes.
Callers supply the bytes directly. The API does not discover or read files.
Duplicate source paths are rejected.

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
evidence. `cargo test -p veln-project` checks these fixed vectors:

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
