# Repair Command Proposal Route

Status: multi-span and multi-file repair target implemented

This page routes remaining repair-command proposal work without requiring the
broad design brief, full open-question inventory, or completed command record
first.

## Read First

- Current advisory repair candidate behavior, candidate input, selection, and
  fail-closed apply gate:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Command syntax and human output gate:
  [../../specification/commands.md](../../specification/commands.md).
- Repair command JSON envelope and command-level candidate shape:
  [../../specification/repair-json.md](../../specification/repair-json.md).

## Current Implemented Boundary

The current implementation supports advisory repair candidates and a narrow
`veln repair` command gate. `repair --apply` can write exactly one safe
unapplied advisory candidate, and that candidate may contain multiple
source-relative replacements in one source file or across multiple source
files. Saved repair JSON input is implemented as a candidate input route, not
as a write authorization by itself.

Use the specification pages above for current behavior. This proposal page is
only for repair-loop behavior that remains outside that boundary.

## Completed Target

The adjacent target for repair candidates that require more than one
replacement is implemented:

- Multi-span repairs: one candidate applies more than one replacement in a
  source file.
- Multi-file repairs: one candidate applies replacements in more than one
  source file.

Use the specification pages above for the implemented multi-edit candidate
shape, saved input matching, atomic write and rollback behavior, and remaining
fail-closed gates.

## Adjacent Work

- Confirmation and override protocol.
- Verification commands beyond the built-in post-edit check analysis.
- Broader ranking models and evidence payloads beyond the advisory candidate
  source preserved in repair JSON.

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
- Use
  [../../reviews/repair-command-completion.md](../../reviews/repair-command-completion.md)
  only when checking the previous command promotion evidence.
- Open
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  only when the specification does not explain the advisory-versus-application
  rationale.
