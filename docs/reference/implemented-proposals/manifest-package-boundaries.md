---
role: implementation-record
authority: supporting
update-when: The completion evidence is superseded, its links become invalid, or current documentation starts relying on this record as authority.
---

# Manifest Package Boundaries

## Summary

This record preserves the completion boundary for editor package-root
selection around package-owned source discovery, command-root selection, and
dependency-root selection. Current behavior is specified in
[commands.md](../../specification/commands.md) and
[editor-support.md](../../specification/editor-support.md). The executable
specification cases named there are the primary evidence.

## Current Boundary

Shared source discovery already stops before descendant directories whose
`veln.toml` is a regular file. It does not follow source or directory symbolic
links. It treats `target` as an ordinary directory, retains the `.git` ignore,
validates explicit input ownership, and supplies the same source set to command
analysis, editor analysis, dependency projects, manifest export validation, and
source-tree checksums.

Command analysis already resolves its invocation directory and selects the
nearest ancestor with a regular manifest marker. It preserves the invocation
directory as the base for relative arguments. Selection failures do not fall
back to a wider or anonymous project.

Source-analysis path dependencies already validate the dependency root's direct
regular manifest before loading its sources.

This record is not current-behavior authority. Use
[commands.md](../../specification/commands.md),
[editor-support.md](../../specification/editor-support.md), and the executable
specification cases for current behavior.

## Terms

- An **analysis start** is the directory from which a command or editor asks to
  select a project.
- A **package root** is the directory supplied to shared project discovery.
- A **manifest package** has a regular `veln.toml` at its package root.
- An **anonymous package** has no `veln.toml` at its package root.

## Completed Editor Behavior

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
When the client supplies a workspace folder through a symbolic-link path, the
deduplicated project keeps that client path valid for diagnostics, overlays,
and navigation requests.

## Acceptance Evidence

The `veln-lsp` server tests mechanically check each acceptance row.

| Case | Input | Required result | Primary evidence |
| --- | --- | --- | --- |
| Manifest workspace root | A workspace root has a manifest and nested packages | Only the outer root is initialized | `server_stops_workspace_root_selection_at_manifest_root` |
| Manifest-free workspace | Separate branches contain manifests | The first manifest root on each branch is initialized | `server_selects_first_manifest_root_on_each_workspace_branch` |
| Explicit nested workspace | The client supplies outer and nested roots | Two deduplicated projects are initialized | `server_keeps_explicit_outer_and_nested_workspace_projects` |
| Dependency workspace isolation | Analysis loads a dependency without a matching workspace folder | The dependency is not initialized as a workspace project | `server_does_not_initialize_loaded_dependency_as_workspace_project` |
| Symlink workspace identity | The client supplies a symlink workspace folder | Alias document URIs remain inside the deduplicated project | `server_keeps_symlink_workspace_alias_documents_in_project` |

## Failure Atomicity

Editor workspace-root selection finishes before source discovery or analysis
starts.

## Non-Goals

- Do not change the implemented owned-source discovery contract.
- Do not add ignore files, manifest exclude patterns, or generic hidden-file
  exclusion.
- Do not define toolchain cache placement or lifecycle; current cache behavior
  is specified in [Commands](../../specification/commands.md).
- Do not change dependency fetching, version selection, or lockfile conflict
  resolution.
- Do not make a nested package an implicit dependency.
- Do not choose when editor diagnostics cover open files, active projects, or
  every workspace project.

## Verification

The relevant guarded test route is:

```sh
bash scripts/agent-test -p veln-lsp
```

The public JSON-RPC evidence for observable branch selection is
[`workspace-package-root-selection`](../../../examples/specification/lsp/workspace-package-root-selection/).

## Completion Boundary

The work completed when editor package-root selection satisfied the acceptance
table, the observable behavior moved to executable evidence and the editor
specification, and this record left the proposal catalog.
