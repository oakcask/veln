# Repair Command Proposal Route

Status: proposed follow-ups

This page routes repair-loop behavior beyond the current `veln repair`
boundary. Implemented repair command records live under
`../../reference/implemented-proposals/`.

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
- Completed proposal records:
  [../../reference/implemented-proposals/repair-command-first-boundary.md](../../reference/implemented-proposals/repair-command-first-boundary.md)
  and
  [../../reference/implemented-proposals/repair-command-confirmation-override.md](../../reference/implemented-proposals/repair-command-confirmation-override.md).

## Current Implemented Boundary

The current implementation supports advisory repair candidates and a narrow
`veln repair` command gate. `repair --apply` can write exactly one safe
unapplied advisory candidate, and that candidate may contain multiple
source-relative replacements in one source file or across multiple source
files. Saved repair JSON input is implemented as a candidate input route, not
as a write authorization by itself.

Use the specification pages above for current behavior. This proposal page now
routes only broader repair-loop behavior that remains outside that boundary.

## Implemented Records

Use the completed records only for history or completion evidence:

- First applying command boundary:
  [../../reference/implemented-proposals/repair-command-first-boundary.md](../../reference/implemented-proposals/repair-command-first-boundary.md).
- Confirmation and explicit override:
  [../../reference/implemented-proposals/repair-command-confirmation-override.md](../../reference/implemented-proposals/repair-command-confirmation-override.md).

Use the source-decision records only for rationale after the current
specification pages do not answer the question:

- [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  for the advisory-versus-application boundary.
- [../../reference/source-decisions/records/result-satisfy-unknown-severity.md](../../reference/source-decisions/records/result-satisfy-unknown-severity.md)
  for the distinction between quiet check diagnostics and strict repair
  authorization.

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
- Open
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  only when the specification does not explain the advisory-versus-application
  rationale.
