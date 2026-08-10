---
role: specification
authority: normative
update-when: The package snapshot identity, portable-domain, capture, digest, or executable-evidence contract changes.
---

# Package Snapshot Digests

`veln-project` captures a package distribution from a filesystem root or from
already embedded manifest and source bytes. It computes a package snapshot
digest from that immutable capture. Callers that only need identity can use
the digest API directly. Duplicate source paths supplied to the digest API are
rejected.

## Portable Package Identity

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

## Embedded Capture

`capture_embedded_package_snapshot` accepts exact manifest bytes and exact
source path and byte pairs. It does not read or materialize a filesystem tree.
It applies the same distribution source exclusions as filesystem capture before
portable-path and source text validation. Therefore `.test.veln` companions,
`_test.veln` integration-test sources, and non-`.veln` inputs are not retained
and cannot fail validation. It sorts retained sources and applies the same
portable-path, UTF-8, case-fold collision, and digest contracts as filesystem
capture. The returned snapshot owns the supplied bytes.

The language server uses this API for the toolchain's embedded `std` manifest
and distribution sources. Therefore the standard package virtual-source
catalog and its source reads refer to one exact retained capture.

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
descendant package boundaries, symbolic links, non-UTF-8 path rejection,
non-UTF-8 excluded symlinks and descendant package roots, non-regular source
rejection, and these fixed digest vectors:

| Manifest and sources | Digest |
| --- | --- |
| Empty manifest; no sources | `f0030b92642915b495c426a5b5185676e0306219a52c448a94fb5e8dccc494ad` |
| Manifest `[package]\nname = "p"\n`; `a.veln` is `a\n`; `z.veln` is `z\n` | `77150b975c9bb56aab9e9b3c8899a81907abc9db535fdfbb6276d40bff9fa878` |
| Empty manifest; `src/λ.veln` is `λ\n` | `f360e18455f6b7c90dd6c34cdec7a444082e003e44583dc8a7d99ae50cba713b` |

The same tests check reversed source order, duplicate paths, isolated changes
to the domain, record tags, byte order, manifest bytes, source path bytes, and
source content bytes. They also prove that equivalent embedded and filesystem
inputs produce identical retained snapshots, that embedded input applies the
same distribution test-source exclusions as filesystem capture, and that
embedded input uses the portable-source validation contract. A Veln source
example is not added for the capture API itself because it is
transport-independent. The editor-facing standard-package use is checked by
the LSP example routed from
[Editor Support](editor-support.md#lsp-navigation-formatting-and-rename).

The same test target is the authoritative Q13 portable-domain matrix. It
checks identity scalar boundaries, identity segments, Unicode whitespace,
identity dot segments, reserved `std`, NFC, portable path segments, controls,
forbidden separators, trailing spaces and dots, device spellings and aliases,
non-UTF-8 source names and text, full default-case-fold collisions, exact
accepted spellings and bytes, reserved device stems followed by ASCII
whitespace before an extension separator, and validation exclusion for
symbolic links, test sources, descendant packages, and `.git` entries.
