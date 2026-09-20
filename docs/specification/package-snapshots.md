---
role: specification
authority: normative
specification-coverage: usage=#filesystem-capture-and-exclusions; behavior=#digest-transcript; limits=#portable-package-identity
update-when: The package snapshot identity, portable-domain, capture, digest, or executable-evidence contract changes.
---

# Package Snapshot Digests

`veln-project` captures a package distribution from a filesystem root or from
already embedded manifest and source bytes. It computes a package snapshot
digest from that immutable capture. Callers that only need identity can use
the digest API directly. Duplicate source paths supplied to the digest API are
rejected.

## Portable package identity

`PackageIdentity` retains the exact validated identity spelling. An ordinary
identity contains 1 through 255 Unicode scalars. It is Unicode Normalization
Form C (NFC), uses nonempty `/`-separated segments, rejects `.` and `..`
segments, and contains no Unicode whitespace. `PackageIdentity::new` rejects `std`.
`PackageIdentity::embedded_standard` is the only API that constructs the
reserved `std` identity.

`PackageIdentityError` distinguishes an empty identity, an identity above the
scalar limit, non-NFC input, an empty segment, a dot segment, a
whitespace-bearing segment, and the reserved standard identity. Validation
does not normalize or rewrite an accepted identity.

## Filesystem capture and exclusions

`capture_package_snapshot` requires a regular `veln.toml` at the supplied
package root. The returned capture retains the manifest's exact bytes, the
ordered owned sources and their exact bytes, and the digest computed from that
same retained data.

An owned source is a regular file whose package-relative path ends in `.veln`.
The capture includes private, non-exported, on-disk generated, and ordinary
`target` sources. It excludes these entries:

- every entry named `.git`, including directory descendants;
- each descendant directory that contains a regular `veln.toml` and all of
  that directory's descendants;
- every symbolic link;
- paths that end in `.test.veln`; and
- paths that end in `_test.veln`.

Source paths use UTF-8 and `/` separators. They are ordered by their UTF-8
bytes. Every retained path is NFC and has nonempty relative segments. A
segment cannot be `.` or `..`, contain a Unicode control, `\`, or `:`, end in
a space or dot, or use a case-insensitive platform-reserved device stem. The
reserved stems are `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM1`
through `COM9`, and `LPT1` through `LPT9`; the superscript aliases for 1, 2,
and 3 are also reserved. ASCII whitespace between the reserved stem and the
extension separator is ignored for this reserved-device check, so
`NUL .veln` is not portable. A suffix after the reserved stem does not make
the segment portable.

Every retained source is valid UTF-8. Two retained source paths cannot have
the same full Unicode default case fold. Unicode normalization and default
case folding use pinned Unicode data version `17.0.0`, exposed as
`PORTABLE_UNICODE_VERSION` and `PORTABLE_UNICODE_VERSION_STRING`.

A discovered path that has no exact UTF-8 representation causes an explicit
capture error unless the entry is excluded before source-path representation
is needed. `PackageSnapshotCaptureError` separately identifies an
unrepresentable source path, an invalid represented source path with its
portable-path reason, invalid source text with its first invalid byte offset,
and a path collision with both exact spellings. A non-regular entry at a
represented distribution source path causes a separate error. Exclusion takes
place before portable validation, so symbolic links, test sources, descendant
packages, and `.git` entries cannot fail capture because of their path spelling
or source bytes.

The capture does not normalize or lossily convert accepted input. It retains
the exact manifest bytes, source bytes, and source-path spellings used by the
digest transcript.

The digest is independent of filesystem enumeration order and the package's
physical parent location. A change to the manifest bytes, an included source
path, or included source bytes changes the digest.

## Embedded capture

`capture_embedded_package_snapshot` accepts exact manifest bytes and exact
source path and byte pairs. It does not read or materialize a filesystem tree.
It filters distribution source suffixes before portable-path and source text
validation. Embedded input does not discover directories, symbolic links, or
descendant manifests; callers supply the distribution source list. Therefore `.test.veln` companions,
`_test.veln` integration-test sources, and non-`.veln` inputs are not retained
and cannot fail validation. It sorts retained sources and applies the same
portable-path, UTF-8, case-fold collision, and digest contracts as filesystem
capture. The returned snapshot owns the supplied bytes.

The language server uses this API for the toolchain's embedded `std` manifest
and distribution sources. Therefore the standard package virtual-source
catalog and its source reads refer to one exact retained capture.

## Digest transcript

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

The low-level `package_snapshot_digest(manifest_bytes, sources)` API preserves
caller-supplied paths and bytes without validating portability or UTF-8 source
text. Use capture for those validations. The digest API rejects duplicate exact
paths and lengths or source counts that cannot fit its `u64` fields.

The package snapshot digest is separate from the lockfile source-tree
checksum. The lockfile checksum retains its `sha256:` prefix and its existing
transcript.

## References

The capture and digest implementations are in
`crates/veln-project/src/snapshot.rs` and
`crates/veln-project/src/snapshot/digest.rs`; portable validation is in
`crates/veln-project/src/portable.rs`. Snapshot tests under
`crates/veln-project/src/snapshot/tests/` cover ordering, exclusions, immutable
bytes, failure boundaries, and fixed digest vectors.
