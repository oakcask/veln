# File Based Modules And Packages

Status: proposed

Veln should derive module identity from package-relative source paths, reserve
`.` for field access and similar expression syntax, use `::` for module paths,
and make `veln.toml` describe package identity plus the modules exported to
other packages.

## Read First

- Current source syntax and module/import behavior:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current name resolution behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current command and manifest behavior:
  [../specification/commands.md](../specification/commands.md).

## Problem

The current module surface splits ownership across source headers and package
metadata:

- Source files may declare `mod` to set the compiler-visible module identity.
- Imported module names use `.` in `use` declarations, while qualified calls
  and public aliases use `::`.
- `veln.toml` may contain `[modules]`, but those entries are metadata and
  cannot rename the source module.
- There is no durable package-level boundary for redistributing a set of
  modules or resolving modules outside the current package.

That makes the authoring model hard to explain. The source file says one
thing, the manifest may list another, and import syntax differs from the path
syntax used elsewhere in source.

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
- Add `[lib].exports` as the manifest list of source modules exported outside
  the package.

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

The imported public surface is limited to declarations and aliases explicitly
public in the target module. Private declarations remain reachable only inside
their defining module.

## External Package Imports

Modules from another package use a package source clause:

```veln
use foo from "github.com/oakcask/foo"
```

The string identifies the external package by its globally unique package
name. The module path before `from` is resolved inside that package, not the
current package. The declaration imports the public names of the exported
module into the current scope.

External imports must resolve only to modules listed by the dependency
package's public export list. A package may contain private helper modules that
are importable from within the same package but unavailable to other packages.

This proposal defines the source-level and manifest-level shape only. Version
selection, package fetching, lockfiles, registry behavior, vendoring, and
authentication are separate package manager design questions.

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

`[lib].exports` lists package-relative `.veln` source paths that are public to
other packages. Each listed file must exist, derive a valid module path, and be
inside the package source tree. The list exports modules, not individual names;
the module's own `pub` declarations define the names external packages can
import.

The manifest does not rename modules. Renaming a module means moving or
renaming the source file.

## Diagnostics

Implementation should report the failed fact at the relevant source or
manifest span:

- `mod` appears in source: the source language no longer accepts module
  headers.
- A `use` path contains `.` as a module delimiter: module paths use `::`.
- A local `use foo::bar` has no matching source file in the current package.
- An external `use foo from "package"` names a package that is unavailable or
  a module that the package does not export.
- A source path segment cannot become a module identifier.
- Multiple files derive the same module path.
- `[modules]` appears in `veln.toml`.
- `[lib].exports` lists a missing file, a non-source file, a duplicate module,
  or a path outside the package source tree.

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

For example, this current shape:

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

When implemented, update:

- `../specification/source-surface.md` and
  `../specification/source-surface-full.md` to remove `ModDecl`, change
  `UseDecl` to `::` module paths, and describe file-based module identity.
- `../specification/names-effects.md` and
  `../specification/names-effects-full.md` to describe local and external
  package import resolution.
- `../specification/commands.md` and `../specification/commands-full.md` to
  replace `[modules]` behavior with `[lib].exports` documentation and
  validation behavior.
- `../../examples/specification/` cases that currently contain `mod`, dotted
  `use` paths, or `[modules]`.

## Acceptance Criteria

- Source files with `mod` declarations are rejected.
- The source paths shown in the module identity example derive module paths
  `foo` and `foo::bar`.
- `use foo::bar` resolves to the corresponding source file in the current
  package and imports public names from that module.
- `use foo from "github.com/oakcask/foo"` resolves module `foo` from the named
  package and imports only public names from an exported module.
- `use foo.bar` is rejected as module-path syntax.
- `veln.toml` rejects `[modules]`.
- `veln.toml` accepts `[package].name` and `[lib].exports`, and validates that
  exported paths derive real package modules.
- Current package private modules are usable by other modules in the same
  package but are not importable from external packages unless exported.

## Open Questions

- Should `use foo::bar` import only bare public names, or should it also bind a
  short alias such as `bar` for qualified paths?
- Should same-package modules require `use` before any qualified access, or
  should fully qualified same-package paths always resolve?
- Should a package have a conventional root module such as `lib.veln`,
  `main.veln`, or a module matching the final package-name segment?
- Should `[lib].exports` list file paths, module paths, or both? File paths
  make export validation direct, while module paths align with source syntax.
- Should package names be opaque strings, URL-like names, or normalized source
  identifiers with a separate registry namespace?
- Should external imports support submodules, for example
  `use sub::module from "github.com/oakcask/foo"`?
- Should import conflicts be rejected at the `use` declaration or only when a
  bare reference is ambiguous?
