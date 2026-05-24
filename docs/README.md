# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reference/implemented-first-slice.md](reference/implemented-first-slice.md)
  is the stable entry point for behavior implemented in the current workspace.
- [reference/language/README.md](reference/language/README.md) routes the
  categorized language specification for the implemented first slice.
- [proposals/README.md](proposals/README.md) routes accepted and open design
  targets that are not fully implemented.
- [reviews/2026-05-24-first-slice-gap-review.md](reviews/2026-05-24-first-slice-gap-review.md)
  is the current gap review against the broader design target.
- [phases/first-slice-implementation.md](phases/first-slice-implementation.md)
  is the current implementation memo for the first slice.
- [discussions/2026-05-24-agent-language-spec-wall.md](discussions/2026-05-24-agent-language-spec-wall.md)
  is the short entry point for the current design-wall discussion based on the
  agent-oriented language proposal.

## Document Status Rules

- Put behavior that works in the current code and tests in `reference/`.
- Put accepted or open design targets that are not fully implemented in
  `proposals/`.
- Put rationale, alternatives, and dated decision history in `discussions/`.
- Put implementation order, completion notes, and working plans in `phases/`.
- Put implementation gaps, verification evidence, and correction lists in
  `reviews/`.
- When a document mixes implemented and planned behavior, either split it or
  label the planned sections and link them from `proposals/`.
- Treat `reference/language/` as the current behavior source before changing
  code, tests, diagnostics, or samples.

Use these status labels at the top of durable specification documents:

- `implemented`: current code and tests support the described behavior.
- `accepted-proposal`: discussions accepted the target, but implementation is
  absent or incomplete.
- `open-proposal`: the design is being explored and should not be treated as a
  commitment.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue this design.

## Conventions

- Put exploratory discussion logs in `discussions/`.
- Put implementation review findings and correction lists in `reviews/`.
- Put stable language reference material in `reference/`.
- Put planned but not fully implemented specification targets in `proposals/`.
- Prefer small, dated files so later agents can read only the relevant context.
- When a discussion accumulates decision results, keep the dated entry file as
  an index and move each result body into a companion detail directory.
