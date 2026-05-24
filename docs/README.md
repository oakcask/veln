# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reference/README.md](reference/README.md) is the stable reference entry
  point for behavior implemented in the current workspace.
- [reference/language/README.md](reference/language/README.md) routes the
  categorized language specification for the implemented first slice.
- [proposals/README.md](proposals/README.md) routes accepted and open design
  targets that are not fully implemented.
- [reviews/first-slice-gap-review.md](reviews/first-slice-gap-review.md)
  is the current gap review against the broader design target.
- [phases/first-slice-implementation.md](phases/first-slice-implementation.md)
  is the current implementation memo for the first slice.
- [reference/source-decisions/README.md](reference/source-decisions/README.md)
  lists discussion decisions that are implemented in the current workspace.
- [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md)
  lists accepted and open design-wall decisions that are not fully implemented.

## Document Status Rules

- Put behavior that works in the current code and tests in `reference/`.
- Put accepted or open design targets that are not fully implemented in
  `proposals/`.
- Put implemented rationale and decision history in `reference/source-decisions/`.
- Put planned or incomplete rationale and decision history in `proposals/`.
- Put implementation order, completion notes, and working plans in `phases/`.
- Put implementation gaps, verification evidence, and correction lists in
  `reviews/`.
- When a proposal becomes fully implemented, move its stable behavior into
  `reference/` and its implemented decision rationale into
  `reference/source-decisions/`; do not leave the implemented proposal under
  `proposals/`.
- When a document mixes implemented and planned behavior, either split it or
  label the planned sections and link them from `proposals/`.
- Treat `reference/language/` as the current behavior source before changing
  code, tests, diagnostics, or samples.

Use these status labels at the top of durable specification documents:

- `implemented`: current code and tests support the described behavior.
- `accepted-proposal`: a decision record accepted the target, but implementation is
  absent or incomplete.
- `open-proposal`: the design is being explored and should not be treated as a
  commitment.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue this design.

## Conventions

- Put exploratory proposal logs in `proposals/`.
- Put implementation review findings and correction lists in `reviews/`.
- Put stable language reference material in `reference/`.
- Put planned but not fully implemented specification targets in `proposals/`.
- Prefer small, dated files so later agents can read only the relevant context.
- When a proposal accumulates implemented decision results, move those result
  bodies into `reference/source-decisions/`.
