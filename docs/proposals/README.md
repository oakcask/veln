# Proposals

This directory keeps design targets that are accepted or being explored but are
not fully implemented in the current workspace.

## Read First

- [grammar-target.md](grammar-target.md) is the accepted first-slice grammar
  target. It includes syntax that is not implemented yet.
- [first-slice-follow-ups.md](first-slice-follow-ups.md) gathers accepted
  first-slice targets that remain incomplete after the completed edit loop.
- [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  lists accepted and open design-wall decisions that are not fully implemented.
- [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  lists current gaps between the implementation and the broader design target.

## Accepted Targets Not Fully Implemented

- Grammar coverage for the remaining full hole grammar is tracked in
  [grammar-target.md](grammar-target.md).
- Remaining first-slice implementation targets are tracked in
  [first-slice-follow-ups.md](first-slice-follow-ups.md).
- Runtime contract enforcement is tracked by
  [Runtime Contract Failure Reporting](agent-language-spec-wall/result-runtime-contract-failure-reporting.md)
  and the current gap review.

## Status Rules

- `implemented` documents belong in `../reference/`.
- Fully implemented proposals must not remain in this directory; move stable
  behavior to `../reference/` and implemented decision rationale to
  `../reference/source-decisions/`.
- `accepted-proposal` means a decision record accepted the target, but code support is
  absent or incomplete.
- `open-proposal` means the design is still exploratory.
- `superseded` documents must link to their replacement.
- `rejected` documents remain only when the negative decision is useful later.

## Read When

- Use this directory when planning the next implementation slice.
- Use `../reference/` when you need current behavior.
- Use `agent-language-spec-wall/` when you need the rationale behind a
  proposal.
- Use `../phases/` when you need implementation order or completion notes.
