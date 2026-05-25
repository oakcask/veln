# Proposals

This directory keeps design targets that are accepted or being explored but are
not fully implemented in the current workspace.

## Read First

- [first-slice-follow-ups.md](first-slice-follow-ups.md) gathers accepted
  first-slice target areas that remain incomplete after the completed edit
  loop and routes to the full details.
- [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  lists accepted and open design-wall decisions that are not fully implemented.
- [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  routes review evidence about first-slice gaps and completion claims.

## Implementation Route

- Choose from accepted targets in
  [first-slice-follow-ups.md](first-slice-follow-ups.md) before opening broader
  design-wall material.
- Compare the target with current behavior in
  [../reference/language/README.md](../reference/language/README.md) so the
  implementation changes only the missing behavior.
- Use [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  for evidence about known gaps before treating a proposal as complete.
- After implementation, promote the resulting behavior into
  `../reference/language/` and keep proposal text only for remaining incomplete
  or historical context. Use [../document-status.md](../document-status.md)
  for the status boundary.

## Accepted Targets Not Fully Implemented

- Remaining first-slice implementation targets are tracked in
  [first-slice-follow-ups.md](first-slice-follow-ups.md), with full detail in
  [first-slice-follow-ups-full.md](first-slice-follow-ups-full.md).
- Future concurrency surface work after the implemented channel and task slices
  is tracked in [first-slice-follow-ups.md](first-slice-follow-ups.md).
- Use [../document-status.md](../document-status.md) before promoting,
  superseding, or rejecting proposal text.

## Read When

- Use this directory when planning the next implementation slice.
- Use `../reference/` when you need current behavior.
- Use `agent-language-spec-wall/` when you need the rationale behind a
  proposal.
- Use `../phases/` when you need implementation order or completion notes.
