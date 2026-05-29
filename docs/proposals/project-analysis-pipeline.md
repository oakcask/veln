# Project Analysis Pipeline

Status: proposed

This page turns the remaining shared-analysis gap from the historical
first-slice review into one implementation target. It is a proposal route, not
current behavior.

## Read First

- Current command behavior: [../specification/commands.md](../specification/commands.md).
- Current execution gates: [../specification/execution.md](../specification/execution.md).
- Current JSON output boundaries:
  [../specification/json-output.md](../specification/json-output.md).
- Proposal implementation mechanics: [implementation-route.md](implementation-route.md).

## Target

Introduce one reusable project-analysis entry point that owns source discovery
results, parse diagnostics, generated doctest sources, surface module loading,
semantic diagnostics, checked-core readiness, and typed-IR readiness.

`check`, `run`, `test`, and `repair` should call that entry point and then apply
only command-specific selection, output, execution, or write policy.

## Non-Goals

- Do not change the documented source discovery rules.
- Do not change current diagnostic ids, JSON envelopes, or human diagnostic
  wording unless a behavioral mismatch is found during implementation.
- Do not expand module loading, import syntax, test discovery, repair
  application, or backend behavior as part of this target.

## Acceptance Checks

- `check`, `run`, `test`, and `repair` use the same project-analysis API for
  parse-clean source loading and semantic analysis.
- Parse errors in one selected file still allow semantic diagnostics from other
  parse-clean selected files.
- Cross-file imports and imported qualified calls use the same facts for
  `check`, `run`, and `test`.
- Checked-core blockers reported by `check` are the same blockers that would
  prevent `run` or `test` from lowering the reachable entry.
- Generated doctests and expected-error doctest reconciliation stay observable
  through the current command behavior.

## Read When

- Use this page for the shared command-analysis target only.
- Use [implementation-route.md](implementation-route.md) before promoting any
  behavior into `../specification/`.
- Use the historical gap classification below only for scope checks or cleanup
  routing.

## Historical Gap Classification

The first-slice gap review found seven areas:

- Shared analysis pipeline: still partially open and captured by this proposal.
- Runtime contract enforcement: current contract pages now define the runtime
  obligation route, so this page does not own that behavior.
- First-slice grammar coverage: the implemented `match` subset is now covered
  by the specification and source support.
- Prelude helper coverage: implemented helper semantics now belong to current
  names, effects, and runtime behavior.
- `veln-test` crate boundary: architectural ownership can be revisited only
  after this shared-analysis entry point exists.
- Captured stdio event fidelity: current test JSON and source-decision records
  now own source-linked output event behavior.
- Executable blockers missing from `check`: partially resolved and retained
  here only as part of shared checked-core readiness.

The historical review also verified that the broad test suite passed, sample
`check --json` and `test --json` commands returned with static diagnostics, and
the previous sample `check` hang was not reproduced.

## Update When

- Move implemented behavior into `../specification/` only when it changes
  observable command behavior.
- Move reusable architecture guidance into `../reference/` after the shared
  entry point exists and tests cover the command parity checks.
