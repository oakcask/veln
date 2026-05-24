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

## Accepted Or Open Targets

- [ADR-Lite Decision Location](result-adr-lite-decision-location.md)
- [Channel-First Concurrency Runtime](result-channel-first-concurrency-runtime.md)
- [Comparison Example Task](result-comparison-example-task.md)
- [Contract Static Runtime Boundary](result-contract-static-runtime-boundary.md)
- [Doctest Error Type Fence Syntax](result-doctest-error-type-fence-syntax.md)
- [Doctest Expected Output Syntax](result-doctest-expected-output-syntax.md)
- [Doctest Result Propagation](result-doctest-result-propagation.md)
- [First-Slice Grammar](result-first-slice-grammar.md)
- [Module Metadata Location](result-module-metadata-location.md)
- [Runtime Contract Failure Reporting](result-runtime-contract-failure-reporting.md)
- [Scoping and Name Resolution](result-scoping-and-name-resolution.md)

## Classification Rule

This directory is for decisions whose accepted behavior is absent, partial, or
only represented as a future compatibility target. When implementation and
tests catch up to a decision, move that record to
`../../reference/source-decisions/` and update both index files.
