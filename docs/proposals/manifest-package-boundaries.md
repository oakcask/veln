---
role: proposal
review-when: The remaining package-root selection scope, acceptance evidence, or implementation status changes.
---

# Manifest Package Boundaries

## Summary

Complete package-root selection around the implemented package-owned source
discovery rule. Current recursive and explicit source ownership is specified in
[commands-full.md](../specification/commands-full.md) and checked by the
executable specification cases named there.

The remaining work selects package roots for command starts, dependency paths,
and editor workspace folders. It does not change the owned-source set once a
consumer supplies a package root.

## Current Boundary

Shared source discovery already stops before descendant directories whose
`veln.toml` is a regular file. It does not follow source or directory symbolic
links. It treats `target` as an ordinary directory, retains the `.git` ignore,
validates explicit input ownership, and supplies the same source set to command
analysis, editor analysis, dependency projects, manifest export validation, and
source-tree checksums.

This implemented behavior is not proposal authority. Use
[commands-full.md](../specification/commands-full.md),
[editor-support.md](../specification/editor-support.md), and the executable
specification cases for current behavior.

## Terms

- An **analysis start** is the directory from which a command or editor asks to
  select a project.
- A **package root** is the directory supplied to shared project discovery.
- A **manifest package** has a regular `veln.toml` at its package root.
- An **anonymous package** has no `veln.toml` at its package root.

## Remaining Proposed Behavior

### Command Package-Root Selection

A command resolves its analysis start to a filesystem identity. It selects the
nearest ancestor directory whose `veln.toml`, inspected without following
symbolic links, is a regular file. If no ancestor qualifies, the resolved
analysis start is an anonymous package root.

An explicit source input does not select another package root. A command rejects
an explicit input that is not owned by the root selected from the analysis
start.

If candidate metadata cannot be classified, selection fails without continuing
to a wider ancestor. A selected unreadable manifest fails when the project
loads it, after root selection.

### Dependency Package-Root Selection

A dependency directory is its package root. The selected directory must contain
a regular `veln.toml` directly. Dependency selection does not search ancestors.

If the marker is absent or does not qualify, dependency loading reports a
command error that identifies the declared dependency path and selected
directory. The dependency remains a separate project and does not become an
editor workspace root unless the client supplies it as one.

### Editor Workspace-Root Selection

An editor resolves each workspace folder before selecting project roots. If the
folder is a manifest package root, the editor selects only that root. Otherwise,
the editor searches each directory branch until it finds the first manifest
package root. It does not search below a selected root. If it finds none, the
workspace folder is one anonymous package root.

Workspace search does not follow directory symbolic links. It skips `.git` and
treats `target` as an ordinary directory. Multiple selected filesystem
identities are sorted and deduplicated. Explicit outer and nested workspace
folders therefore produce separate projects without combining their sources.

## Acceptance Model

The following rows describe only the remaining work. The evidence column names
evidence to add; it does not claim that the evidence passes now.

| Case | Input | Required result | Planned primary evidence |
| --- | --- | --- | --- |
| Command below manifest root | A command starts below a package manifest | The nearest qualifying ancestor is the package root | CLI harness case |
| Anonymous command start | No qualifying ancestor manifest exists | The resolved analysis start is an anonymous package root | CLI harness case |
| Symlinked analysis start | Direct and symbolic paths identify one directory | Both select the same package identity | Platform-conditional project-root test |
| Classification failure | Candidate metadata cannot be classified | Selection fails without ancestor fallback | Fault-injected project-root test |
| Unreadable selected manifest | The selected regular manifest cannot be read | Selection succeeds and project loading reports the read failure | Platform-conditional project-loading test |
| Dependency root | A dependency directory contains `veln.toml` directly | The directory is selected without ancestor search | Dependency test and checked case |
| Dependency descendant without manifest | An ancestor has a manifest but the selected dependency directory does not | Loading rejects the selected directory | Dependency test and CLI harness case |
| Manifest workspace root | A workspace root has a manifest and nested packages | Only the outer root is initialized | LSP project-root case |
| Manifest-free workspace | Separate branches contain manifests | The first manifest root on each branch is initialized | LSP project-root case |
| Explicit nested workspace | The client supplies outer and nested roots | Two deduplicated projects are initialized | LSP multi-root case |
| Dependency workspace isolation | Analysis loads a dependency without a matching workspace folder | The dependency is not initialized as a workspace project | LSP dependency case |

## Failure Atomicity

Package-root selection must not continue with a wider or partial project after
a filesystem classification error. Dependency selection must not discover or
analyze sources before it validates the selected dependency root.

## Non-Goals

- Do not change the implemented owned-source discovery contract.
- Do not add ignore files, manifest exclude patterns, or generic hidden-file
  exclusion.
- Do not define toolchain cache placement or lifecycle; [Toolchain User
  Cache](toolchain-user-cache.md) owns that work.
- Do not change dependency fetching, version selection, or lockfile conflict
  resolution.
- Do not make a nested package an implicit dependency.
- Do not choose when editor diagnostics cover open files, active projects, or
  every workspace project.

## Verification

Implementation must add the evidence named in the remaining acceptance model.
The relevant guarded test routes are:

```sh
bash scripts/agent-test -p veln-project
bash scripts/agent-test -p veln-lsp
bash scripts/agent-test -p veln-cli --test toolchain_harness
```

## Completion Boundary

This proposal is complete when command, dependency, and editor package-root
selection satisfy the remaining acceptance model and current behavior is
promoted to executable evidence and the matching specification pages. Then move
the record to `../reference/implemented-proposals/` and remove it from the
proposal catalog.
