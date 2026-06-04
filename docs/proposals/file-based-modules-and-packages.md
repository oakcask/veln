# File Based Modules And Packages

Status: proposed

Veln should derive module identity from package-relative source paths, reserve
`.` for field access and similar expression syntax, use `::` for module paths,
and make `veln.toml` describe package identity plus the modules exported to
other packages.

The local-source, manifest-export, path-dependency external-import, and first
package-manager slices are implemented: source `mod` declarations are
rejected, selected source paths derive same-package module identity, local
`use` declarations use `::`, same-package qualified access requires a matching
written import, `[modules]` is rejected, `[lib].exports` validates selected
package source-file exports, `use path from "package"` resolves exported
modules from already available path dependencies, git dependency metadata
records one `rev`, `tag`, or `branch` selector plus optional `subdir`, and
`veln package lock` writes deterministic lockfile entries for already
available path dependencies and local git dependencies with `rev`, `tag`, or
`branch` selectors. This proposal remains open for package-manager behavior
beyond local path and local git lockfile slices.

## Read First

- Current source syntax and module/import behavior:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current name resolution behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current command and manifest behavior:
  [../specification/commands.md](../specification/commands.md).

## Problem

Earlier module work split ownership across source headers and package
metadata:

- Source files may declare `mod` to set the compiler-visible module identity.
- Imported module names use `.` in `use` declarations, while qualified calls
  and public aliases use `::`.
- `veln.toml` could contain `[modules]`, but those entries were metadata and
  could not rename the source module.
- There is no durable package-level boundary for redistributing a set of
  modules or resolving modules outside the current package.

The implemented local-source, manifest-export, path-dependency
external-import, and first package-manager metadata slices remove those local
ambiguities and establish the first external package boundary. This proposal
remains open for fetching, vendoring, non-local source materialization, mirror
support, and graph-wide incompatible-source resolution.

## Proposal

Replace source-declared module identity with file-based module identity and add
package manifests as the redistribution boundary.

- Remove `mod` declarations from the source language.
- Treat each `.veln` source file as exactly one module.
- Derive a module path from its package-relative file path.
- Use `::` as the module path delimiter everywhere in source.
- Keep `.` for field access and other expression-local member syntax.
- Introduce packages as the unit that redistributes multiple modules.
- Require each package manifest to declare a globally unique package name.
- Remove `[modules]` from `veln.toml`.
- Add `[lib].exports` as the manifest list of package-relative source files
  exported outside the package.

## Module Identity

A package-relative source path determines the module path:

```text
foo.veln
foo/bar.veln
foo/bar/baz.veln
```

Those files implement these modules:

```text
foo
foo::bar
foo::bar::baz
```

The module path is not written inside the source file. A source file containing
`mod foo` or any other `mod` declaration is rejected once this proposal is
implemented.

Path segments must be valid Veln module identifiers. A file whose stem or
directory segment cannot be represented as a module identifier is rejected as a
source module. File-system paths are normalized before module derivation, and
two files that derive the same module path are a package error.

## Local Imports

`use` declarations use module paths:

```veln
use foo::bar
```

For a source file in the current package, this declaration resolves
`foo::bar` to the package-relative source file represented by that module path.
It imports the public names of that module into the current scope.

Name conflicts are errors when a bare reference would be ambiguous. A local
declaration or binding shadows an imported public name for ordinary bare name
resolution. The imported module path remains available for qualified access, so
callers may write a fully qualified path when that is clearer or when a bare
name would conflict.

Import conflict checks are intentionally delayed until a bare reference needs
the imported name. A `use` declaration remains valid when its public names
overlap with another imported module, the implicit prelude, or a local
declaration. This permits modules to import broad public surfaces for qualified
access without requiring authors to pre-resolve every possible bare-name
collision. The conflict becomes a diagnostic only when the current source uses
the shared name bare and no local declaration or binding shadows the imported
names.

The imported public surface is limited to declarations and aliases explicitly
public in the target module. Private declarations remain reachable only inside
their defining module.

`use foo::bar` does not also bind a short module alias such as `bar`.
Qualified access uses the imported module path itself, such as
`foo::bar::name`. This keeps `use` from synthesizing extra names beyond the
public names it imports and avoids ambiguity when multiple imported module
paths share the same final segment.

Same-package qualified access also requires a written `use` declaration. A
module may not reach another same-package module solely by spelling the full
module path at the use site. This keeps dependency edges explicit, lets source
selection follow imports instead of arbitrary qualified references, and matches
the rule that qualified calls do not fall back to unrelated bare names.

## External Package Imports

Modules from another package use a package source clause:

```veln
use foo from "github.com/oakcask/foo"
use sub::module from "github.com/oakcask/foo"
```

The string identifies the external package by its globally unique package
name. The module path before `from` is resolved inside that package, not the
current package. The declaration imports the public names of the exported
module into the current scope.

External imports may name exported submodules. The `from` clause selects only
the package; it does not imply a package-root prefix and does not rewrite the
module path. For example, `use sub::module from "github.com/oakcask/foo"`
resolves module `sub::module` inside package `github.com/oakcask/foo`.

External imports must resolve only to modules listed by the dependency
package's public export list. A package may contain private helper modules that
are importable from within the same package but unavailable to other packages.

External imports are source-level references to already declared package
dependencies. They do not choose versions, fetch packages, authorize network
access, or bypass dependency metadata. Package manager metadata supplies the
source and lockfile facts described in
[Package Manager Implications](#package-manager-implications).

Package names are URL-like globally unique identity strings. The identity is
stable source-level metadata, not necessarily a direct fetch URL. A package
manager may resolve a package name through git remotes, dependency metadata,
mirrors, lockfiles, or vendored sources, but source imports continue to name the
package identity.

## Package Manifest

`veln.toml` declares package identity and public module exports:

```toml
[package]
name = "github.com/oakcask/foo"

[lib]
exports = [
  "foo.veln",
  "bar.veln",
]
```

`[package].name` is the globally unique package identity used by external
imports. The value is a string and must be present for a multi-module package
that is intended to be redistributed.

The package name should be URL-like so authors can create decentralized names
without a central package registry. For example, a package may use a name such
as `github.com/oakcask/foo` or `codeberg.org/team/lib` while the package
manager records the concrete git remote, selected revision, and verification
data separately. A fetched package's manifest name must match the identity that
requested it.

`[lib].exports` lists package-relative `.veln` source paths that are public to
other packages. It does not accept module paths, and a manifest must not mix
file paths with module paths. Each listed file must exist, derive a valid
module path, and be inside the package source tree. The list exports modules,
not individual names; the module's own `pub` declarations define the names
external packages can import.

The manifest does not rename modules. Renaming a module means moving or
renaming the source file.

Packages do not have an implicit or conventional root module. A file named
`lib.veln`, `main.veln`, or with the same stem as the final package-name
segment is an ordinary module unless it is listed in `[lib].exports` and
imported by its derived module path. Authors that want a root-like public
module should create and export the corresponding source file explicitly.

File paths are the manifest format because the manifest describes the package's
redistribution surface over concrete source files. The compiler derives module
paths from those files using the same rule as local source discovery, which
keeps export validation tied to existence, path containment, duplicate module
derivation, and invalid path segments. Accepting module paths in the manifest
would create a second spelling for the same export surface without adding a
rename mechanism.

## Diagnostics

Implementation should report the failed fact at the relevant source or
manifest span:

- `mod` appears in source: the source language no longer accepts module
  headers.
- A `use` path contains `.` as a module delimiter: module paths use `::`.
- A local `use foo::bar` has no matching source file in the current package.
- A qualified same-package path names a module that is not imported by a
  written `use` declaration.
- A bare reference could resolve to public names from multiple imported
  modules and is not shadowed by a local declaration or binding.
- An external `use path from "package"` names a package that is unavailable or
  a module path that the package does not export.
- A source path segment cannot become a module identifier.
- Multiple files derive the same module path.
- `[modules]` appears in `veln.toml`.
- `[lib].exports` lists a missing file, a non-source file, a module path, a
  duplicate module, or a path outside the package source tree.

Related notes should point to the conflicting source file, manifest entry,
export list, or imported module when that context is available.

## Migration

The migration path is mechanical:

- Remove each `mod` declaration.
- Move or rename source files so package-relative paths match the desired
  module paths.
- Rewrite `use a.b` to `use a::b`.
- Replace `[modules]` with `[lib].exports` for modules that should be visible
  outside the package.
- Keep private helper modules out of `[lib].exports`.

For example, this earlier shape:

```text
src/foo.veln
src/bar.veln
veln.toml
```

```veln
mod app.foo
use app.bar
```

```toml
[modules]
"src/foo.veln" = "app.foo"
"src/bar.veln" = "app.bar"
```

becomes:

```text
foo.veln
bar.veln
veln.toml
```

```veln
use bar
```

```toml
[package]
name = "github.com/oakcask/app"

[lib]
exports = [
  "foo.veln",
  "bar.veln",
]
```

## Specification Updates

Implemented package import, dependency-metadata, and local lockfile behavior is
specified under `../specification/source-surface.md`,
`../specification/names-effects.md`, `../specification/commands.md`, and
`../../examples/specification/`. Remaining work belongs to package-manager
behavior: non-local dependency source materialization, vendoring, mirror
support, and graph-wide incompatible-source resolution.

## Package Manager Implications

The implemented path-dependency and local git lockfile workflows are
specified under `../specification/commands.md` and executable cases under
`../../examples/specification/package/`. The remaining package-manager work is
non-local source materialization.

External imports do not authorize network fetching by themselves. Future
non-path package manager commands should require dependency metadata that maps
a package identity to a concrete source, such as a git remote, version
selector, revision, checksum, vendored directory, or mirror. Lockfile entries
for those sources should record the resolved source separately from the
package identity so the source-level import path remains stable when the
retrieval route changes.

This keeps Veln packages decentralized: a project can depend on git-hosted
packages without publishing through a crate-style central registry, while still
letting package manager tooling verify that the resolved package declares the
expected `[package].name`.

Dependency table keys are package identities. The key must be the same string
that source imports name in `from "package"` clauses, and the resolved
package's `[package].name` must match that key. Source locations are not
identities: a git remote, mirror, vendored directory, or local path only
describes where the package manager finds the package with that identity.

```toml
[dependencies."github.com/oakcask/foo"]
git = "https://github.com/oakcask/foo.git"
tag = "v1.2.0"

[dependencies."github.com/oakcask/bar"]
git = "https://github.com/oakcask/mono.git"
branch = "main"
subdir = "packages/bar"

[dependencies."github.com/oakcask/baz"]
path = "../baz"
```

A git dependency must name a git remote and exactly one selector: `rev`, `tag`,
or `branch`. The selector is the requested source. The implemented local git
lockfile slice resolves the selector to the revision used for builds, so
mutable selectors such as branches do not make a checked-in build depend on
the current remote state.

`subdir` is an optional package root inside the fetched repository. It lets a
single repository publish multiple Veln packages while preserving each
package's own identity and manifest. The package manager validates
`[package].name` inside the selected subdirectory, not at the repository root
unless `subdir` is absent.

Git lockfile entries use package identities as primary keys and store the
resolved source separately:

```toml
[[package]]
name = "github.com/oakcask/bar"
source = {
  kind = "git",
  url = "https://github.com/oakcask/mono.git",
  selector = { rev = "abc123" },
  rev = "...",
  subdir = "packages/bar",
}
checksum = "sha256:..."
```

Every git lockfile entry update generates a checksum for the package source
tree that Veln will compile. The checksum verifies the package contents after
source selection, including any `subdir`; it does not replace the resolved git
revision. A package manager may use mirrors or vendored storage to obtain the
source, but the lockfile entry must still preserve the package identity,
resolved source, and checksum needed to verify the materialized package.

The initial resolver should select at most one package instance for a package
identity. If the dependency graph requires incompatible revisions for the same
identity, resolution fails instead of loading multiple copies under one source
import name.
