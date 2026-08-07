---
role: proposal
review-when: The package-boundary acceptance model, source-discovery scope, or implementation status changes.
---

# Manifest Package Boundaries

## Summary

Define `veln.toml` as a source-discovery boundary. A Veln package owns source
files below its package root, but it does not own source files below a nested
directory that contains another `veln.toml`.

Commands, editor analysis, dependency loading, and source-tree checksums must
apply the same ownership rule. Tools must not combine independently manifested
packages merely because their directories are nested.

This proposal changes planned project and command behavior. The current
implemented behavior remains documented in
[commands-full.md](../specification/commands-full.md) and
[editor-support.md](../specification/editor-support.md).

## Motivation

Current source discovery recursively selects `.veln` files below the current
project root. It skips `.git` and directories named `target`, but it does not
stop at a nested `veln.toml`. As a result, a command or editor session can
interpret sources from a nested package as local modules of the outer package.

The `target` exclusion also prevents a package from using that ordinary name
for source organization. The current command specification places reusable JVM
class cache entries below the selected user cache root, so source ownership
does not need to reserve `target` as a toolchain output directory.

The manifest already identifies package metadata, exports, and dependencies.
It must also establish which package owns a discovered source. Without that
boundary, module identity and manifest validation can depend on the directory
from which a larger tree is analyzed.

## Terms

- An **analysis start** is the directory from which a command, editor, or
  dependency operation requests project analysis.
- A **package root** is the nearest directory at or above the analysis start
  that contains `veln.toml`. If the analysis start and its ancestors contain
  no `veln.toml`, the analysis start is the package root.
- A **manifest package** is a package whose package root contains
  `veln.toml`.
- An **anonymous package** is a package whose package root does not contain
  `veln.toml`.
- A **nested manifest root** is a proper descendant directory of the current
  package root that contains `veln.toml`.
- A **package boundary** is the boundary between the current package and a
  nested manifest root.
- A source is **owned by the current package** when it is below the current
  package root and no directory from the source's parent through the current
  package root, excluding the current package root itself, is a nested
  manifest root.

A `veln.toml` candidate establishes a package boundary exactly when its
filesystem type, inspected without following symbolic links, is a regular file.
The file's readability and manifest validity do not affect this classification.
A hard link to a regular file therefore qualifies. A directory, symbolic link,
or other non-regular object does not qualify.

A missing candidate does not establish a boundary. If a tool cannot determine
whether a candidate exists and is a regular file, project-root selection or
source discovery must fail. The tool must not continue ancestor search, descend
through the candidate directory, or return a partial or wider package.

A tool does not open or parse a nested manifest before it recognizes the
boundary. An unreadable nested regular file therefore excludes its subtree
without making outer-package discovery fail. If that directory is selected as
the package root, loading its own unreadable manifest fails after root
selection.

## Proposed Behavior

### Package Root Selection

A tool resolves the analysis start to its filesystem identity before it
selects a package root. A symlink through which the analysis start was reached
does not create a second package identity.

A CLI command uses its resolved working directory as its analysis start. It
searches that directory and its ancestors for the nearest `veln.toml`. If it
finds a manifest, the directory containing that manifest is the package root.
If it does not find a manifest, the resolved working directory is an anonymous
package root.

An explicit source input does not replace the CLI analysis start and does not
select another package root.

An editor uses each resolved editor project directory as an analysis start.
It may resolve multiple manifest package roots inside a larger workspace, but
each resolved root remains a separate project. An editor must not combine the
owned sources of those projects.

For each editor workspace folder, the editor initializes the outermost package
roots visible from that folder. If the folder itself is a manifest package
root, the editor initializes that root and does not search below its package
boundaries. If the folder is not a manifest package root, the editor searches
each directory branch until it finds the first manifest package root and does
not search below that root. If the search finds no manifest package root, the
workspace folder remains one anonymous project root.

This workspace-root search does not follow directory symbolic links. It keeps
the `.git` ignored-directory rule and gives a directory named `target` no
special treatment.

A nested manifest root is initialized separately only when an editor project
directory resolves to that root, such as when a client supplies it as another
workspace folder. Explicit outer and nested project directories produce two
separate projects. Resolved filesystem identities are sorted and deduplicated.
A manifest dependency does not become an editor project merely because
dependency analysis loads it.

Dependency resolution uses the resolved dependency directory as the selected
package root. The selected directory must contain `veln.toml` directly.
Dependency analysis must not search ancestors for a manifest. The selected
manifest establishes that dependency's package root.

### Path Identity and Symbolic Links

Package ownership uses the resolved filesystem identity of the package root.
Tools may retain the user-supplied path for diagnostics, but that path does not
create another package or source identity.

Before a tool checks an explicit input against the package root, it must
lexically remove `.` components and apply `..` components. An input whose `..`
components escape the package root must produce a command-level discovery
error.

A symlink below the package root is not an owned source or directory. Recursive
discovery must not select the symlink and must not follow it. This rule applies
even when the symlink target is inside the same package. It also applies when a
directory symlink targets another package or creates a cycle.

A symlink named `veln.toml` does not establish a package boundary. This rule
applies to valid and dangling symlinks.

An explicit input must produce a command-level discovery error when the input
itself, or a path component below the package root, is a symlink. The result
does not depend on whether the symlink target exists or whether the target is
inside or outside the package root.

The package root itself may be reached through a symlink. Resolving the
analysis start before package-root selection makes an invocation through that
symlink equivalent to an invocation through the resolved directory.

### Recursive Discovery

When a tool recursively discovers sources for a package, it selects `.veln`
files owned by that package. The tool must not descend into a nested manifest
root.

A directory without a qualifying `veln.toml` boundary marker does not create a
package boundary. Its descendant `.veln` files remain owned by the current
package unless another ancestor of those files is a nested manifest root.

The existing `.git` ignored-directory rule remains independent of package
boundaries. A directory named `target` receives no special treatment. Owned
`.veln` files below it are selected, and a qualifying `veln.toml` boundary
marker below it establishes the same nested package boundary as a marker at any
other owned path.

This proposal does not add another ignored-directory mechanism. Toolchain
cache placement does not change source ownership, including when the cache
location is overridden to a path below a package root.

Discovery must sort and deduplicate the selected source paths after applying
the ownership rule.

### Explicit Inputs

Every explicit source file or source directory input must be owned by the
current package. An input that is outside the current package root or inside a
nested manifest root must produce a command-level discovery error.

The error must identify the rejected input. For an input inside a nested
manifest root, the error must also identify that nested package root. The
command must not parse or analyze a partial source selection when any explicit
input crosses a package boundary.

An explicit input does not change the current package root. A user analyzes a
nested package by invoking the command with that nested directory as the
package root, not by selecting its sources from the outer package.

### Manifest Loading

A project reads only the `veln.toml` at its package root as its own manifest.
A nested manifest is not validated as part of the outer package. A malformed
nested manifest therefore does not make discovery of the outer package fail.

Manifest exports do not add files to the current package's selected source
set. Each export must continue to name a source owned by the manifest package.

### Dependency Packages

When dependency resolution selects a dependency package root, the tool creates
a separate project for that root. Source discovery for the dependency applies
the same package-boundary rule relative to the dependency package root.

If the selected dependency directory does not contain `veln.toml` directly,
dependency loading must produce a command-level dependency error before it
discovers or analyzes dependency sources. A `veln.toml` in an ancestor of the
selected directory does not satisfy this requirement. The error must identify
the declared dependency path and the selected directory where `veln.toml` was
required.

A nested package is not a dependency merely because it is below the current
package root. The current package can consume it only through supported
manifest dependency metadata and package import behavior.

Declarations from a dependency package do not become local declarations of
the importing package. Existing export and visibility checks continue to
control which dependency declarations an import can use.

### Editor Analysis

An editor project rooted at a manifest package must use the same owned-source
set as command analysis rooted at that package. An editor may initialize
multiple manifest package roots, but it must analyze each root as a distinct
project.

This requirement does not determine when the editor publishes diagnostics.
It only determines the package context and source set used when analysis runs.

### Source-Tree Checksums

A source-tree checksum for a package includes only `.veln` files owned by that
package. It excludes sources below nested manifest roots. A nested dependency
package receives its own checksum when dependency resolution selects it.

Adding, removing, or changing a source below a nested manifest root must not
change the outer package's source-tree checksum.

## Acceptance Model

The following table is the planned behavioral contract. The evidence column
names evidence to add during implementation; it does not describe tests that
already pass.

| Case | Package layout or action | Required result | Planned primary evidence |
| --- | --- | --- | --- |
| Manifest package discovery | The package root contains a manifest, one source at the root, and another source below `src` | Discovery selects both sources | `veln-project` discovery unit test |
| Command below manifest root | A command runs from `src` below a package manifest | The nearest ancestor manifest defines the package root and discovery uses package-relative source paths | CLI harness case |
| Nested manifest boundary | A `vendor` descendant named `lib` contains its own manifest and source | Outer discovery selects neither nested package file | `veln-project` boundary unit test and a checked specification case |
| Deep nested boundary | A nested manifest root occurs below directories without manifests | Discovery stops at the directory containing `veln.toml` | `veln-project` boundary unit test |
| Ordinary `target` directory | The package contains `local.veln` below `target` and no manifest below that directory | Discovery selects the source as owned | `veln-project` discovery unit test and a checked specification case |
| Nested package below `target` | A `lib` descendant below `target` contains its own manifest and source | Outer discovery excludes the nested source because of the manifest boundary, not the directory name | `veln-project` boundary unit test and an LSP project-root case |
| Invalid nested manifest | The nested `veln.toml` is malformed | Outer discovery still succeeds and excludes the nested tree | Checked specification case |
| Unreadable nested manifest | A nested regular-file `veln.toml` cannot be opened for reading | Outer discovery excludes the nested tree without opening the marker | Platform-conditional `veln-project` boundary unit test |
| Unreadable selected manifest | The same unreadable regular-file manifest is selected as the package root | Root selection succeeds and manifest loading reports the read failure | Platform-conditional `veln-project` project-loading unit test |
| Directory named `veln.toml` | A directory contains a non-manifest directory with that name and otherwise owned sources | The object does not establish a boundary and normal discovery continues | `veln-project` boundary unit test |
| Other non-regular marker | A supported host exposes a non-regular, non-symlink object named `veln.toml` | The object does not establish a boundary and is not opened | Platform-conditional `veln-project` boundary unit test |
| Boundary classification failure | Candidate metadata inspection returns an error other than not-found | Root selection or discovery fails without ancestor fallback, descent, or a partial source set | Fault-injected `veln-project` unit test |
| Explicit nested file | An outer-package command explicitly selects the source below the nested `vendor` package | The command fails with a discovery error before source analysis | CLI harness case |
| Explicit nested directory | An outer-package command explicitly selects `vendor/lib` | The command fails with the same boundary class | CLI harness case |
| Input outside the root | A command explicitly selects a source outside the current package root | The command fails without analyzing a partial selection | CLI harness case |
| Parent path escapes root | An explicit input uses `..` components that leave the current package root | The command reports a discovery error without analyzing a partial selection | `veln-project` discovery unit test and CLI harness case |
| Implicit source symlink | Recursive discovery encounters a `.veln` symlink below the package root | Discovery excludes the symlink even when its target is an owned source | Platform-conditional `veln-project` discovery unit test |
| Implicit directory symlink | Recursive discovery encounters a directory symlink, including one that enters another package or forms a cycle | Discovery does not descend through the symlink and terminates without selecting sources through it | Platform-conditional `veln-project` discovery unit test |
| Symlinked manifest marker | A directory below the package root contains a valid or dangling symlink named `veln.toml` | The symlink does not establish a package boundary | Platform-conditional `veln-project` boundary unit test |
| Explicit symlink | An explicit source or directory input is a symlink or contains a symlink component below the package root | The command reports a discovery error whether the target is internal, external, or dangling | Platform-conditional `veln-project` unit test and CLI harness case |
| Symlinked analysis start | Equivalent analysis starts name the same directory directly and through a symlink | Both select the same resolved package root and owned-source identities | Platform-conditional `veln-project` project-root test |
| Nested package invocation | Analysis uses `vendor/lib` as its package root | The nested manifest and `lib.veln` form an independent project | Checked specification case |
| Anonymous outer package | An anonymous package contains local sources and a nested manifest root | Discovery selects the anonymous package's sources and excludes the nested package | Checked specification case |
| Manifest export crosses boundary | The outer manifest exports a source below a nested manifest root | Manifest validation rejects the export as outside the package's owned source set | Checked specification case |
| Declared dependency | The outer manifest declares the nested package through supported dependency metadata and imports an exported module | Dependency analysis uses a separate project rooted at the dependency manifest | Checked specification case |
| Dependency package root | A dependency path names a directory that contains `veln.toml` directly | Dependency loading selects that directory as the package root without searching its ancestors | `veln-project` dependency-loading unit test and a checked specification case |
| Dependency path below package root | A dependency path names a descendant without `veln.toml`, while an ancestor of that directory contains `veln.toml` | Dependency loading reports a command-level dependency error and does not select the ancestor package | `veln-project` dependency-loading unit test and CLI harness case |
| Package checksum isolation | A nested package source changes without changing an outer-package source | The outer checksum remains equal and the nested checksum changes | `veln-project` checksum unit test |
| Command and editor parity | A command and editor analysis use the same package root and saved files | Both analyze the same owned-source paths | LSP project test and CLI harness case |
| Manifest workspace root | An editor workspace folder is an outer manifest package containing nested manifests | The editor initializes only the outer project and its source set excludes every nested package | LSP project-root case |
| Manifest-free workspace | A workspace folder has no manifest at its root and contains sibling manifest packages, one with a deeper nested package | The editor initializes the first manifest root on each branch and does not initialize the deeper package | LSP project-root case |
| Explicit nested workspace root | A client supplies both an outer manifest root and one of its nested manifest roots as workspace folders | The editor initializes two deduplicated projects and does not combine their source sets | LSP multi-root project case |
| Dependency is not a workspace root | An outer editor project loads a nested manifest package through dependency metadata without a separate workspace folder | Dependency analysis remains separate and the editor does not initialize another workspace project | LSP project-root and dependency case |

## Failure Atomicity

If one explicit input crosses a package boundary, source discovery fails for
the complete invocation. The tool must not return a project containing only
the remaining valid inputs. Commands must not emit generated code, formatted
files, repair edits, documentation output, or a lockfile from that partial
selection.

This rule makes a mixed-package invocation observable as one discovery failure
instead of an order-dependent partial analysis.

## Compatibility Consequences

Commands without explicit paths will stop selecting sources below nested
manifest roots. Commands that currently analyze sources from more than one
manifest package in one invocation will need separate invocations or supported
dependency metadata.

Commands and editor analysis will start selecting owned `.veln` files below
directories named `target`. A nested manifest below `target` will become
visible to project-root discovery. Packages that contain generated or foreign
`.veln` files there must move them outside the package, place them below a
nested manifest boundary, or select narrower explicit inputs.

A command run below a manifest package root will use the nearest ancestor
manifest instead of treating its working directory as an anonymous package
root. Its source paths and selected no-input source set can therefore change.

Explicit cross-boundary paths that are currently accepted by discovery will
become command errors. Module, manifest, and semantic diagnostics caused only
by combining nested packages will disappear.

Package source-tree checksums can change when a package currently contains
nested manifest roots or owned `.veln` files below a directory named `target`.
Lockfile evidence must be updated only when the new checksum input set is
intentional and verified.

## Non-Goals

- Do not define fixture-specific behavior for `examples/specification` or any
  other repository directory.
- Do not add `.velnignore`, manifest exclude patterns, or generic hidden-file
  exclusion.
- Do not change the existing `.git` ignored-directory rule.
- Do not define the toolchain cache location or remaining cache recovery
  lifecycle in this proposal; [Commands](../specification/commands.md)
  specifies the current location, and
  [Toolchain User Cache](toolchain-user-cache.md) owns the remaining recovery
  work.
- Do not define dependency fetching, version selection, or lockfile conflict
  resolution.
- Do not change package export or visibility semantics beyond requiring an
  export to remain inside its package boundary.
- Do not choose whether editor diagnostics cover open files, active projects,
  or every workspace project.
- Do not make a nested package an implicit dependency.

## Verification

Implementation must add the evidence named in the acceptance model. Planned
local verification uses repository-relative commands:

```sh
bash scripts/agent-test -p veln-project
bash scripts/agent-test -p veln-lsp
bash scripts/agent-test -p veln-cli --test toolchain_harness
```

The full toolchain harness must verify that existing commands outside the
changed package-boundary cases preserve their outputs and side effects.

## Completion Boundary

This proposal is complete only when all source-discovery consumers apply the
same package ownership rule, the acceptance evidence passes, and current
behavior is promoted to the matching pages under `../specification/` and
`../../examples/specification/`.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
