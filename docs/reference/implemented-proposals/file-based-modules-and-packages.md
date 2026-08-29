---
role: implementation-record
authority: supporting
update-when: File-based module identity, package manifest, external package import, or package lock source behavior changes.
---

# File Based Modules And Packages

This record preserves the completed proposal history for file-based module
identity, package manifests, external package imports, and package lock source
resolution. Current behavior is specified under `../../specification/` and
covered by executable examples under `../../../examples/specification/`.

## Implemented Behavior

- Source `mod` declarations are rejected.
- Selected source paths derive same-package module identity.
- Local `use` declarations use `::`.
- Same-package qualified access requires a matching written import.
- `[modules]` is rejected.
- `[lib].exports` validates selected package source-file exports.
- `use path from "package"` resolves exported modules from already available
  path dependencies.
- Git dependency metadata records one `rev`, `tag`, or `branch` selector plus
  optional `subdir`.
- `veln package lock` writes deterministic lockfile entries for already
  available path dependencies, local git dependencies, materialized non-local
  git dependencies, already available vendored package directories, and
  already materialized mirror source trees.
- `veln package lock` follows dependency manifests across the graph and rejects
  incompatible source selections for the same package identity before writing a
  lockfile.

## Current Specification Routes

- Source syntax, path-derived module identity, manifests, dependency metadata,
  and package exports: [../../specification/source-surface.md](../../specification/source-surface.md).
- Name resolution, local imports, and external package imports:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Package lock command behavior:
  [../../specification/commands.md](../../specification/commands.md) and
  [../../specification/commands.md](../../specification/commands.md).

## Executable Evidence

- Source surface and manifest behavior is covered under
  `examples/specification/check/`.
- Package lock behavior is covered under `examples/specification/package/`.

## Completion Notes

The proposal is complete because the remaining package-manager behavior is now
implemented: dependency table keys remain package identities, graph traversal
selects at most one source for each identity, and incompatible path, vendor,
mirror, git selector, or git subdirectory selections are rejected before
`veln.lock` is written.
