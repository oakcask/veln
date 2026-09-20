---
role: specification
authority: normative
update-when: The veln package lock command input handling, dependency materialization, lockfile update, or cleanup behavior changes.
specification-coverage: usage=#package-lock-command; behavior=#resolution; limits=#limits-and-errors
---

# Package Lock Command

Use `veln package lock` to resolve the current project's dependency tables
and write `veln.lock`. The supported source forms are a string `path`,
`vendor`, or `mirror`, or a string `git` plus exactly one of `rev`,
`tag`, or `branch`. Registry sources are not resolved.

## Resolution

Dependency table keys are package identities in the portable identity domain.
Each identity may resolve to one compatible source selection. Repeated
selections must agree on source kind, location, git selector, and `subdir`.
A later incompatible selection fails with
`package.incompatible_dependency_source` and does not write a lockfile.

Path, vendor, and mirror roots must exist, contain a direct `veln.toml`, and
have a matching `[package].name`. A git source may be a local repository,
local `file:` URL, or non-local URL. Non-local repositories are materialized
under the package cache before checkout. `subdir` selects the repository
relative package root, whose manifest name must match.

## Lockfile and checksum

The lockfile contains sorted `[[package]]` entries. Each entry records
`name`, a source object, and a `sha256:` checksum. Source paths use `/`
separators. Path, vendor, and mirror entries preserve their source kind; git
entries also record URL, selector, resolved commit, and optional subdirectory.

The checksum covers sorted owned `.veln` files below the resolved package root,
using source-boundary and ignored-directory rules. Descendant package roots and
`.git` contents are excluded; `target` is an ordinary source directory.
Equivalent lexical root spellings produce the same normalized source path and
checksum.

## Limits and errors

An invalid package identity fails at its dependency key with
`package.invalid_dependency_identity`. Manifest-name mismatches fail before
writing. Materialization or checkout failures fail without publishing a partial
lockfile. Existing lockfile content is not overwritten by a failed resolution.

## References

Implementation: `crates/veln-cli/src/commands/package.rs`. Package-lock
integration coverage verifies source forms, normalization, checksums,
conflicting selections, and refusal paths.
