---
role: specification
authority: normative
update-when: The veln package lock command input handling, dependency materialization, lockfile update, or cleanup behavior changes.
---

# Package Lock Command

`package lock` reads the current project `veln.toml`, follows dependency
tables in resolved dependency manifests, and writes `veln.lock`. The
implemented package-manager slice supports dependency tables with exactly one
source field: a string-valued `path` field, a string-valued `vendor` field,
string-valued `mirror` field naming an already materialized source tree, or a
string-valued `git` field plus exactly one selector: `rev`, `tag`, or `branch`.
The command materializes non-local git URLs through git before lockfile
generation. It does not resolve registry sources.

Dependency table keys are package identities. `package lock` rejects a
dependency table key that is outside the portable package identity domain
specified by [package-snapshots.md](package-snapshots.md). A rejected key
reports `package.invalid_dependency_identity` at the dependency key and
refuses to write `veln.lock`. The checked
`../../examples/specification/package/package-lock-dot-segment-identity/` case
shows this rejection for a key with a `..` segment.

Across the graph, a package identity may resolve to only one source selection.
Repeated dependencies on the same identity are compatible when the source kind,
source location, requested git selector, and git `subdir` match after lockfile
path normalization. If a later dependency table selects a different source
location, source kind, git selector, or git `subdir` for an identity that was
already selected, `package lock` reports
`package.incompatible_dependency_source` at the later dependency key, adds a
related note for the first dependency key, and refuses to write `veln.lock`.

For each path dependency, the dependency table key is the package identity.
The command requires the path to name an existing package root, reads that
root's `veln.toml`, and requires its `[package].name` to match the dependency
table key before writing an entry. A mismatch is reported at the dependency
table key with a related note on the dependency manifest name when available.

The written lockfile uses sorted `[[package]]` entries for the resolved
dependency graph. Each entry records the package `name`, a path `source`
object, and a `sha256:` checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "path", path = "vendor/lib" }
checksum = "sha256:..."
```

Serialized source paths use `/` separators. The checksum is computed from the
sorted owned `.veln` source files discovered under the dependency package root
after the same package-boundary and ignored-directory rules as source
discovery. Descendant package roots and `.git` contents do not affect the
lockfile. A directory named `target` is an ordinary source directory, so owned
`.veln` files below `target` do affect the lockfile. Lexically equivalent
dependency root spellings use the same package-relative source path names when
computing the checksum. The checked case `lock-normalized-path-dependency`
proves that a path dependency spelled with a `..` component writes the
normalized source path and computes the checksum from owned sources below that
normalized root.

For each vendor dependency, the dependency table key is the package identity
and `vendor` names an already available vendored package directory. The
command reads that directory's `veln.toml`, requires its `[package].name` to
match the dependency table key, and writes a distinct vendor source record:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "vendor", path = "vendor/lib" }
checksum = "sha256:..."
```

Vendor lockfile entries use the same source-tree checksum rule as path
dependencies. The distinct source kind preserves that the source came from
vendored package storage rather than an ordinary local path dependency.

For each mirror dependency, the dependency table key is the package identity.
The command requires `mirror` to name an already materialized package source
tree, reads that tree's `veln.toml`, and requires its `[package].name` to match
the dependency table key before writing an entry. The written mirror source
record preserves the package identity separately from the mirror source path
and checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "mirror", path = "mirror/github.com/oakcask/lib" }
checksum = "sha256:..."
```

For each git dependency, the `git` value may name an already available local
repository path, a local `file:` URL, or a non-local git URL. Non-local URLs are
materialized under `.veln/package/git/` before the requested selector is
resolved. Existing materialized repositories are fetched before checkout. If
`subdir` is present, the command uses that repository-relative package root for
manifest validation and checksum generation. The dependency package root must
contain `veln.toml`, and its `[package].name` must match the dependency table
key.

The written git source record stores the package identity separately from the
source URL, requested selector, resolved commit, optional subdirectory, and
source-tree checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = {
  kind = "git",
  url = "vendor/mono",
  selector = { branch = "main" },
  rev = "0123456789abcdef0123456789abcdef01234567",
  subdir = "packages/lib",
}
checksum = "sha256:..."
```

