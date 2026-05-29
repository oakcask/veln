# Agent-Language Spec Wall Proposals

Status: proposed

This directory keeps design-wall material that is not fully implemented in the
current workspace. Implemented decisions were moved to
`../../reference/source-decisions/`, and completed proposal records were moved
to `../../reference/implemented-proposals/`.

## Read First

- [repair-command.md](repair-command.md) routes repair-loop behavior beyond the
  current advisory candidate and repair command specification pages.
- Completed repair-command records:
  [../../reference/implemented-proposals/repair-command-first-boundary.md](../../reference/implemented-proposals/repair-command-first-boundary.md)
  and
  [../../reference/implemented-proposals/repair-command-confirmation-override.md](../../reference/implemented-proposals/repair-command-confirmation-override.md).
- [design-brief.md](design-brief.md) routes to the broad thesis and
  first-slice design anchors; open
  [design-brief-full.md](design-brief-full.md) only for the original brief.
- [open-questions.md](open-questions.md) routes resolved and unresolved
  questions; open [open-questions-full.md](open-questions-full.md) only when
  auditing the full inventory.
- [../../reference/source-decisions/records/result-adr-lite-decision-location.md](../../reference/source-decisions/records/result-adr-lite-decision-location.md)
  records the implemented ADR-lite comment decision.

## Proposed Targets

- This directory as a whole is broad context; use or create one short proposal
  page for implementation work.
- Implemented grammar, concurrency, effect-label, and comparison-task decisions
  now live under `../../reference/`.
- Broader repair verification commands, ranking models, partial application,
  and general automatic repair behavior need a new short proposal page before
  they become targets.
- The advisory repair candidate boundary is implemented: `check --json`
  candidates stay unapplied, carry target and evidence records, include
  explicit blocking obligations, and do not authorize automatic application.
- CLI parser coverage keeps `veln repair` and `veln help repair` outside the
  advisory-only boundary until the dedicated repair command specification is
  used.
- Use [../../reference/source-decisions/README.md](../../reference/source-decisions/README.md)
  before opening old decision records directly.

## Classification Rule

This directory is for decisions whose proposed behavior is absent, partial, or
only represented as a future compatibility target. When implementation and
tests catch up to a decision, move that record to
`../../reference/source-decisions/` or
`../../reference/implemented-proposals/` and update the matching index files.
