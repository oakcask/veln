# Package Lockfile Sources

Status: implemented

This record covers the completed package-manager lockfile source slice from
file-based modules and packages. Current behavior lives in
`../../specification/`; this page is only history and completion evidence.

## Implemented Behavior

- `veln package lock` reads dependency tables keyed by package identity.
- Path, vendored, mirror, and git dependency sources are materialized or
  validated from explicit manifest metadata, not from source-level
  `use ... from "package"` declarations.
- Lockfile entries keep package identity separate from source facts.
- Lockfile entries include a deterministic checksum for the selected package
  source tree.
- Dependency package manifests must declare a `[package].name` that matches the
  requested package identity.
- Non-local git dependencies are materialized through git before lockfile
  generation and record the resolved revision.
- Lockfile generation follows dependency manifests across the graph and rejects
  incompatible source selections for repeated package identities before
  writing `veln.lock`.

## Current Specification

- Command behavior:
  `../../specification/commands.md`.
- Full `veln package lock` reference:
  `../../specification/commands-full.md#veln-package-lock`.
- Manifest dependency metadata:
  `../../specification/source-surface.md`.

## Executable Evidence

- `../../../examples/specification/package/lock-path-dependencies/`.
- `../../../examples/specification/package/lock-vendor-dependency/`.
- `../../../examples/specification/package/lock-mirror-dependency/`.
- `../../../examples/specification/package/lock-git-rev-dependency/`.
- `../../../examples/specification/package/lock-git-tag-dependency/`.
- `../../../examples/specification/package/lock-git-branch-dependency/`.
- `../../../examples/specification/package/lock-git-remote-rev-dependency/`.
- `../../../examples/specification/package/lock-git-remote-tag-dependency/`.
- `../../../examples/specification/package/lock-git-remote-branch-dependency/`.
- `../../../examples/specification/package/lock-package-name-mismatch/`.
- `../../../examples/specification/package/lock-vendor-package-name-mismatch/`.
- `../../../examples/specification/package/lock-mirror-package-name-mismatch/`.
- `../../../examples/specification/package/lock-mirror-unavailable/`.
- `../../../examples/specification/package/lock-incompatible-transitive-path-source/`.
- `../../../examples/specification/package/lock-incompatible-transitive-git-selector/`.
- `../../../examples/specification/package/lock-incompatible-transitive-source-kind/`.
- `../../../examples/specification/package/lock-incompatible-transitive-git-subdir/`.

## Boundary

- Multiple versions of the same package identity are not loaded.
- Registry or network policy beyond explicit git metadata is outside this
  slice.

## Skip Unless Needed

- Do not read this page for current command behavior.
- Use this record only when auditing why package lockfile source support is no
  longer listed as active proposal work.
