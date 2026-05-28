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
