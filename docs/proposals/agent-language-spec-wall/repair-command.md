# Repair Command Proposal Route

Status: proposed

This page routes the repair-loop proposal area without requiring the broad
design brief or full open-question inventory first.

## Read First

- Current advisory repair candidate behavior and the absence of an implemented
  repair command:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Safe repair candidate boundary:
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md).

## Current Boundary

The current implemented boundary is
[advisory repair candidates](../../specification/repair-candidates.md).
This proposal starts where that boundary stops: command invocation,
confirmation, override, and automatic application remain unresolved proposal
work.

## Open Command-Level Work

- Final command name and invocation shape.
- Candidate schema outside the current diagnostic `details` payload.
- Edit representation for multi-file or multi-span repairs.
- Ranking model and evidence payload.
- Confirmation, override, and verification protocol.

## Completion Conditions

This proposal is not implementation-ready until the command-level work above is
resolved into explicit acceptance criteria. A future implementation can promote
this proposal only when all of these behavior points are specified and covered:

- The invocation shape is fixed, including whether the entry point is
  `repair`, an option on `check`, or another command.
- The input source for candidates is defined, including whether the command
  consumes existing advisory candidates, computes candidates itself, or accepts
  a machine-readable candidate file.
- The candidate and edit schema is defined outside the diagnostic `details`
  payload, including source-relative targets, replacement text, ranking
  evidence, known limits, blocking obligations, verification hints, and
  application policy.
- The write policy is fixed, including the default dry-run or apply behavior,
  confirmation requirements, override recording, and whether partial multi-file
  edits are allowed.
- The fail-closed cases are listed, including missing candidates, ambiguous
  targets, stale spans, parse or check failures after editing, unknown hard
  obligations, and verification commands that fail or cannot run.
- The human output and JSON output are both specified, including what is stable
  machine-readable behavior and what remains advisory context.
- The implementation has command tests for preview, application, refusal,
  JSON output, stale target handling, verification failure, and preservation of
  the existing advisory `check --json` boundary.
- The implemented behavior is documented under `../../specification/`, and any
  remaining unresolved repair-loop behavior stays in proposal text.

## Review Handoff

The completion conditions are not met. Current implementation and
specification still stop at advisory candidates:

- `crates/veln-cli/src/cli.rs` exposes `check`, `fmt`, `run`, `test`,
  `explain`, and `lsp`, with no `repair` command or repair option.
- `crates/veln-cli/tests/check_json.rs` asserts that `repair` and
  `help repair` are unknown commands.
- `crates/veln-cli/tests/check_json.rs` also asserts that
  `safe_repair_candidate` edits remain unapplied after `check --json`.
- `../../specification/commands.md`,
  `../../specification/commands-full.md`, and
  `../../specification/repair-candidates.md` state that no applying repair
  command is implemented and that candidate edits remain advisory.

Next work should first turn the open command-level points into explicit
acceptance criteria. Implementation should then add command tests for the
preview, apply, refusal, JSON, stale-target, verification-failure, and existing
`check --json` advisory-boundary cases before promoting any behavior into
`../../specification/`.

Until these conditions are resolved, adding an applying `veln repair` command
would be premature. Work may still improve advisory candidate records, but it
must keep edits unapplied and follow the current specification boundary.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Promoting repair command behavior into current implementation.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `explain`, or a future command.

## Skip Unless Needed

- Use [open-questions.md](open-questions.md) only when auditing the historical
  design-wall inventory.
- Use [design-brief.md](design-brief.md) only when the broad repair-loop thesis
  is needed.
