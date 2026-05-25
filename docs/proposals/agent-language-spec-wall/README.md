# Agent-Language Spec Wall Proposals

Status: open-proposal

This directory keeps design-wall material that is not fully implemented in the
current workspace. Implemented decisions were moved to
`../../reference/source-decisions/`.

## Read First

- [design-brief.md](design-brief.md) gives the broad thesis and first-slice
  design anchors.
- [open-questions.md](open-questions.md) routes resolved and unresolved
  questions.
- [../grammar-target.md](../grammar-target.md) is the consolidated accepted
  grammar target; it intentionally includes syntax beyond the current parser
  and backend.
- [../../reference/source-decisions/result-adr-lite-decision-location.md](../../reference/source-decisions/result-adr-lite-decision-location.md)
  records the implemented ADR-lite comment decision.

## Accepted Or Open Targets

- [Channel-First Concurrency Runtime](result-channel-first-concurrency-runtime.md)
- [First-Slice Grammar](result-first-slice-grammar.md)

Implemented rationale such as
[Comparison Example Task](../../reference/source-decisions/result-comparison-example-task.md)
lives in the reference decision index.

## Classification Rule

This directory is for decisions whose accepted behavior is absent, partial, or
only represented as a future compatibility target. When implementation and
tests catch up to a decision, move that record to
`../../reference/source-decisions/` and update both index files.
