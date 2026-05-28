# Repair Command Proposal Route

Status: implemented saved candidate input boundary; adjacent work proposed

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

Candidate input can come from current source analysis or saved repair JSON
input. Selecting a candidate by current `repair_id`, saved command-level id, or
`source_candidate_id` is current behavior documented in
[../../specification/repair-candidates.md](../../specification/repair-candidates.md).

## Completed Target: Saved Candidate Inputs

The repair command can consume saved candidate inputs instead of requiring the
displayed candidate set to come only from source analysis.

For implementation work, keep two ideas separate:

- Candidate selection: choosing one current command-level candidate by
  `repair_id` or `source_candidate_id`.
- Candidate input: where the command-level candidates come from. Saved
  candidate files belong here and are implemented for repair JSON inputs.

The implemented boundary preserves the fail-closed application gate: a saved
candidate input is not a write authorization by itself, and stale, ambiguous,
non-applicable, or unsupported candidates refuse rather than apply. Current
behavior is specified in
[../../specification/repair-candidates.md](../../specification/repair-candidates.md).

## Adjacent Work

- Multi-file or multi-span repairs.
- Confirmation and override protocol.
- Verification commands beyond the built-in post-edit check analysis.
- Broader ranking models and evidence payloads beyond the advisory candidate
  source preserved in repair JSON.

## Command Detail

Use [repair-command-full.md](repair-command-full.md) only when auditing the
implemented first command completion record. Use this page for adjacent
command-level work.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Implementing adjacent repair candidate input routes beyond saved repair JSON.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `explain`, or a future command.

## Skip Unless Needed

- Use [open-questions.md](open-questions.md) only when auditing the historical
  design-wall inventory.
- Use [design-brief.md](design-brief.md) only when the broad repair-loop thesis
  is needed.
- Use [repair-command-full.md](repair-command-full.md) only when auditing the
  completed command criteria.
