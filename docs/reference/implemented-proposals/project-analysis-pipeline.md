# Project Analysis Pipeline

Status: implemented

This page records the implemented shared-analysis target from the historical
first-slice review. Use the specification pages for current command and editor
behavior.

## Read First

- Current command behavior:
  [../../specification/commands.md](../../specification/commands.md).
- Current editor and LSP behavior:
  [../../specification/editor-support.md](../../specification/editor-support.md).
- Current execution gates:
  [../../specification/execution.md](../../specification/execution.md).
- Current JSON output boundaries:
  [../../specification/json-output.md](../../specification/json-output.md).
- Use this page for completion evidence and cleanup routing only.

## Outcome

The shared project-analysis entry point owns source discovery results, parse
diagnostics, generated doctest sources, surface module loading, semantic
diagnostics, checked-core readiness, and typed-IR readiness.

The reusable implementation lives in the internal `veln-analysis` crate so
tooling can call the same analysis path without depending on `veln-cli`.

`check`, `run`, `test`, and `repair` call that entry point and then apply only
command-specific selection, output, execution, or write policy.

`veln lsp` also uses that entry point for workspace diagnostics. It excludes
doctest-generated sources, overlays unsaved editor buffers, and keeps semantic
token requests document-scoped as specified by
`../../specification/editor-support.md`.

This record is now history and routing. New shared-analysis work should use a
new proposal page unless it is already stated by `../../specification/`.

## Boundary

- Do not change the documented source discovery rules.
- Do not change current diagnostic ids, JSON envelopes, or human diagnostic
  wording unless a behavioral mismatch is found during implementation.
- Do not expand module loading, import syntax, test discovery, repair
  application, or backend behavior as part of this target.
- Do not use this historical record as the source for current LSP behavior;
  route to `../../specification/editor-support.md`.

## Completion Evidence

- `check`, `run`, `test`, and `repair` use the same project-analysis API for
  parse-clean source loading and semantic analysis.
- `veln lsp` uses the same project-analysis API for discovered workspace file
  diagnostics while preserving document-scoped semantic tokens.
- Parse errors in one selected file still allow semantic diagnostics from other
  parse-clean selected files.
- Cross-file imports and imported qualified calls use the same facts for
  `check`, `run`, and `test`.
- Cross-file workspace diagnostics in LSP use the same checked-project facts as
  `check`.
- Checked-core blockers reported by `check` are the same blockers that would
  prevent `run` or `test` from lowering the reachable entry.
- Generated doctests and expected-error doctest reconciliation stay observable
  through the current command behavior.

## Read When

- Checking why the shared command-analysis target is no longer listed as
  active.
- Reviewing completion evidence before removing or superseding this route.
- Use the historical gap classification below only for scope checks or cleanup
  routing.

## Historical Gap Classification

The first-slice gap review found seven areas:

- Shared analysis pipeline: implemented and retained here as completion
  evidence.
- Runtime contract enforcement: current contract pages now define the runtime
  obligation route, so this page does not own that behavior.
- First-slice grammar coverage: the implemented `match` subset is now covered
  by the specification and source support.
- Prelude helper coverage: implemented helper semantics now belong to current
  names, effects, and runtime behavior.
- `veln-test` crate boundary: architectural ownership can be revisited only
  through a separate proposal target.
- Captured stdio event fidelity: current test JSON and source-decision records
  now own source-linked output event behavior.
- Executable blockers missing from `check`: partially resolved and retained
  here only as history for shared checked-core readiness.

The historical review also verified that the broad test suite passed, sample
`check --json` and `test --json` commands returned with static diagnostics, and
the previous sample `check` hang was not reproduced.

## Update When

- New command-analysis work becomes implemented and belongs in the current
  specification or a separate proposal route.
- This historical route stops carrying useful completion evidence.
