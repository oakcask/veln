# Repair Command Proposal Route

Status: confirmation and override target implemented

This page records the implemented repair-command confirmation and explicit
override target without requiring the broad design brief, full open-question
inventory, or completed command record first.

## Read First

- Current advisory repair candidate behavior, candidate input, selection, and
  fail-closed apply gate:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Command syntax and human output gate:
  [../../specification/commands.md](../../specification/commands.md).
- Repair command JSON envelope and command-level candidate shape:
  [../../specification/repair-json.md](../../specification/repair-json.md).
- Current proposal-level target status:
  [../target-selection.md](../target-selection.md).

## Current Implemented Boundary

The current implementation supports advisory repair candidates and a narrow
`veln repair` command gate. `repair --apply` can write exactly one safe
unapplied advisory candidate, and that candidate may contain multiple
source-relative replacements in one source file or across multiple source
files. Saved repair JSON input is implemented as a candidate input route, not
as a write authorization by itself.

Use the specification pages above for current behavior. This proposal page now
routes only broader repair-loop behavior that remains outside that boundary.

## Completed Target

The confirmation and override protocol for `veln repair` is implemented.

The implemented target is scoped to explicit user confirmation and explicit
override recording around repair application:

- `--confirm CANDIDATE_ID` records the id the user confirmed before writing.
- `--override` requires `--confirm` and can apply one confirmed
  `manual_review_required` candidate.
- Override records the accepted application policy, status, and advisory
  blocking obligations in repair JSON.
- Override keeps the normal source-relative target, stale-span, hole-target,
  overlap, rollback, and post-edit verification gates.

Read these current-behavior anchors before proposing or implementing the
next target:

- [../../specification/repair-candidates.md](../../specification/repair-candidates.md)
  for advisory candidate input, selection, and the implemented apply gate.
- [../../specification/repair-json.md](../../specification/repair-json.md)
  for preview, apply, refusal, and verification records.
- [../../specification/commands.md](../../specification/commands.md) for the
  implemented command boundary.
- [../../specification/holes.md](../../specification/holes.md) and
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md)
  when candidate safety, hole spans, diagnostic details, or advisory candidate
  records are involved.

Use the source-decision records only for rationale after the current
specification pages do not answer the question:

- [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  for the advisory-versus-application boundary and future override rationale.
- [../../reference/source-decisions/records/result-satisfy-unknown-severity.md](../../reference/source-decisions/records/result-satisfy-unknown-severity.md)
  for the distinction between quiet check diagnostics and strict repair
  authorization.

## Previously Completed Target

The adjacent target for repair candidates that require more than one
replacement is implemented:

- Multi-span repairs: one candidate applies more than one replacement in a
  source file.
- Multi-file repairs: one candidate applies replacements in more than one
  source file.

Use the specification pages above for the implemented multi-edit candidate
shape, saved input matching, atomic write and rollback behavior, and remaining
fail-closed gates.

The earlier command-promotion target also completed saved candidate input,
command-level repair JSON output, safe candidate selection, stale-span and
target validation, overlap refusal, rollback, and post-edit check verification.
Saved candidate input is a way to choose candidates; it is not write
authorization by itself.

## Deferred Adjacent Work

- Verification commands beyond the built-in post-edit check analysis.
- Broader ranking models and evidence payloads beyond the advisory candidate
  source preserved in repair JSON.
- Partial application and general automatic repair behavior.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `explain`, or a future command.

## Skip Unless Needed

- Use [open-questions.md](open-questions.md) only when auditing the historical
  design-wall inventory.
- Use [design-brief.md](design-brief.md) only when the broad repair-loop thesis
  is needed.
- Use [repair-command-full.md](repair-command-full.md) only when auditing the
  completed command criteria.
- Open
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  only when the specification does not explain the advisory-versus-application
  rationale.
