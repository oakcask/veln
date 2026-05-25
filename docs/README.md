# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reference/README.md](reference/README.md): current implemented behavior.
- [proposals/README.md](proposals/README.md): accepted or open work not yet
  fully implemented.
- [reviews/README.md](reviews/README.md): gaps and verification evidence.

## Read When

- Changing syntax, types, effects, contracts, holes, commands, JSON output, or
  examples: start at [reference/language/README.md](reference/language/README.md).
- Planning the next slice: start at [proposals/README.md](proposals/README.md),
  then check [reviews/README.md](reviews/README.md).
- Explaining why implemented behavior exists: use
  [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Reconstructing implementation order: use
  [phases/README.md](phases/README.md).
- Reading incomplete design-wall rationale: use
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).

## Current Shape

- `reference/language/` is the first stop for implemented language behavior.
- `reference/source-decisions/` is historical rationale grouped by topic.
- `phases/` and `reviews/` contain longer working records behind short indexes.
- `proposals/` contains accepted or open targets that still need promotion into
  the reference after implementation.

## Skip Unless Needed

- Do not read old phase plans before the current reference and review pages.
- Do not read source-decision records when the language reference answers the
  behavior question.
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
