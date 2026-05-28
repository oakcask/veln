# Agent-Language Spec Wall Proposals

Status: proposed

This directory keeps design-wall material that is not fully implemented in the
current workspace. Implemented decisions were moved to
`../../reference/source-decisions/`.

## Read First

- [repair-command.md](repair-command.md) routes the implemented repair command
  record, current advisory candidate behavior, and remaining command-level
  work; open [repair-command-full.md](repair-command-full.md) only for
  completion and handoff criteria.
- [design-brief.md](design-brief.md) routes to the broad thesis and
  first-slice design anchors; open
  [design-brief-full.md](design-brief-full.md) only for the original brief.
- [open-questions.md](open-questions.md) routes resolved and unresolved
  questions; open [open-questions-full.md](open-questions-full.md) only when
  auditing the full inventory.
- [../../reference/source-decisions/records/result-adr-lite-decision-location.md](../../reference/source-decisions/records/result-adr-lite-decision-location.md)
  records the implemented ADR-lite comment decision.

## Proposed Targets

- Implemented grammar, concurrency, effect-label, and comparison-task decisions
  now live under `../../reference/`.
- Unresolved design-wall targets may be chosen as proposal work when the
  current specification does not already cover the behavior.
- Completion review:
  [../../reviews/agent-language-spec-wall-completion.md](../../reviews/agent-language-spec-wall-completion.md).
- Use [../../reference/source-decisions/README.md](../../reference/source-decisions/README.md)
  before opening old decision records directly.

## Classification Rule

This directory is for decisions whose proposed behavior is absent, partial, or
only represented as a future compatibility target. When implementation and
tests catch up to a decision, move that record to
`../../reference/source-decisions/` and update both index files.
