# File Based Modules And Packages

Status: proposed

This proposal remains open only for package-manager behavior that is not yet
implemented: graph-wide incompatible-source resolution.

Implemented behavior for file-based module identity, `::` imports,
`[lib].exports`, path dependency imports, dependency manifest metadata, and
`veln package lock` entries for path, git, vendor, and mirror dependencies is
current specification behavior. Do not use this proposal as the authority for
those implemented slices.

## Read First

- Current source syntax, package manifests, and dependency metadata:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current name resolution, local imports, and external package imports:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current `veln package lock` behavior:
  [../specification/commands.md](../specification/commands.md).

## Implemented Boundary

The implemented local-source, manifest-export, path-dependency
external-import, and package-lock slices are specified outside this proposal:

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
- Path, git, vendor, and mirror lockfile entries preserve package identity
  separately from resolved source information and the source-tree checksum.

## Remaining Problem

Package identities are stable source-level metadata, but source locations are
not identities. A git remote, mirror, vendored directory, or local path only
describes where tooling finds the package with that identity.

The implemented package manager can lock one available source per dependency
table entry. It does not yet detect incompatible source selections across a
dependency graph before writing a lockfile.

## Remaining Proposal

Future package-manager work should keep package identity separate from source
retrieval details.

Dependency table keys remain package identities. The key must be the same
string that source imports name in `from "package"` clauses, and the resolved
package's `[package].name` must match that key before the source is accepted.

Graph-wide incompatible-source resolution should select at most one package
instance for a package identity. If the dependency graph requires
incompatible revisions or sources for the same identity, resolution should fail
instead of loading multiple copies under one source import name.

## Completion Conditions

- Graph-wide incompatible-source diagnostics are specified under
  `../specification/` and covered by executable examples when implemented.
- This proposal is moved to `../reference/implemented-proposals/` only after
  the remaining package-manager behavior is implemented and documented as
  current behavior.
