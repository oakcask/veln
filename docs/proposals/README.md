# Proposals

This directory keeps design targets that are accepted or being explored but are
not fully implemented in the current workspace.

## Read First

- [grammar-target.md](grammar-target.md) is the accepted first-slice grammar
  target. It includes syntax that is not implemented yet.
- [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  lists current gaps between the implementation and the broader design target.

## Accepted Targets Not Fully Implemented

- Grammar coverage: `match`, explicit `test` declarations, and full hole
  grammar are tracked in [grammar-target.md](grammar-target.md).
- Runtime contract enforcement is tracked by
  [Runtime Contract Failure Reporting](../discussions/agent-language-spec-wall/result-runtime-contract-failure-reporting.md)
  and the current gap review.
- Prelude helper coverage is tracked by
  [First-Slice Prelude Helpers](../discussions/agent-language-spec-wall/result-first-slice-prelude-helpers.md)
  and the current gap review.
- Captured stdio event fidelity is tracked by
  [Stdio API and Output Events](../discussions/agent-language-spec-wall/result-stdio-api-and-output-events.md)
  and the current gap review.

## Status Rules

- `implemented` documents belong in `../reference/`.
- `accepted-proposal` means discussions accepted the target, but code support is
  absent or incomplete.
- `open-proposal` means the design is still exploratory.
- `superseded` documents must link to their replacement.
- `rejected` documents remain only when the negative decision is useful later.

## Read When

- Use this directory when planning the next implementation slice.
- Use `../reference/` when you need current behavior.
- Use `../discussions/` when you need the rationale behind a proposal.
- Use `../phases/` when you need implementation order or completion notes.
