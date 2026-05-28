# Repair Command Proposal Route

Status: implemented first command boundary; saved candidate inputs proposed

This page routes the promoted repair-command boundary and the remaining
command-level proposal work without requiring the broad design brief or full
open-question inventory first.

## Read First

- Current advisory repair candidate behavior, application-policy boundary, and
  implemented command gate:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Implemented command availability:
  [../../specification/commands.md](../../specification/commands.md).
- Implemented repair JSON output:
  [../../specification/repair-json.md](../../specification/repair-json.md).
- Completion review for this command promotion:
  [../../reviews/repair-command-completion.md](../../reviews/repair-command-completion.md).
- Safe repair candidate rationale, only when the specification does not explain
  the boundary:
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md).

## Current Implemented Boundary

The implemented boundary is
[advisory repair candidates](../../specification/repair-candidates.md) plus a
narrow `veln repair` command gate. `repair` previews command-level candidate
records and `repair --apply` can apply exactly one safe unapplied advisory
candidate after post-edit check verification.

Candidate input is recomputed from current source analysis. Saved candidate
files and other external candidate inputs are not current behavior.

## Active Target: Saved Candidate Inputs

The current remaining target is to let the repair command consume saved
candidate inputs instead of requiring every invocation to recompute candidates
only from source analysis.

Keep this target subordinate to the implemented specification until code and
tests promote it. The proposal work must preserve the current fail-closed
application gate: a saved candidate input is not a write authorization by
itself, and stale, ambiguous, non-applicable, or unsupported candidates must
still refuse rather than apply.

## Adjacent Work

- Multi-file or multi-span repairs.
- Confirmation and override protocol.
- Verification commands beyond the built-in post-edit check analysis.
- Broader ranking models and evidence payloads beyond the advisory candidate
  source preserved in repair JSON.

## Command Detail

Use [repair-command-full.md](repair-command-full.md) only when auditing the
implemented completion record. Use this page for the saved candidate input
target and adjacent command-level work.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Implementing saved candidate files or other repair candidate inputs.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `explain`, or a future command.

## Skip Unless Needed

- Use [open-questions.md](open-questions.md) only when auditing the historical
  design-wall inventory.
- Use [design-brief.md](design-brief.md) only when the broad repair-loop thesis
  is needed.
- Use [repair-command-full.md](repair-command-full.md) only when auditing the
  completed command criteria.
