# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reference/README.md](reference/README.md) is the stable reference entry
  point for behavior implemented in the current workspace.
- [proposals/README.md](proposals/README.md) routes accepted and open design
  targets that are not fully implemented.
- [reviews/README.md](reviews/README.md) routes implementation gap reviews and
  verification evidence.

## Read When

- Use [reference/language/README.md](reference/language/README.md) before
  changing code, tests, diagnostics, or samples.
- Use [phases/README.md](phases/README.md) for implementation order,
  completion notes, and working plans.
- Use [reference/source-decisions/README.md](reference/source-decisions/README.md)
  for implemented discussion results and compatibility context.
- Use [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md)
  for accepted or open design-wall decisions that are not fully implemented.

## Skip Unless Needed

- Do not read old phase plans before the current reference and review pages.
- Do not read source-decision records when the categorized language reference
  answers the behavior question.
- Do not treat proposal text as implemented behavior unless the reference also
  states it.

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
- Prefer small, purpose-labeled files so later agents can read only the relevant
  context.
- When a proposal accumulates implemented decision results, move those result
  bodies into `reference/source-decisions/`.
