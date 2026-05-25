# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reference/README.md](reference/README.md): implemented behavior.
- [proposals/README.md](proposals/README.md): accepted or open targets that
  are not fully implemented.
- [reviews/README.md](reviews/README.md): implementation gaps and verification
  evidence.

## Read When

- Use [reference/language/README.md](reference/language/README.md) before code,
  test, diagnostic, or sample changes.
- Use [phases/README.md](phases/README.md) for implementation order and working
  plans.
- Use [reference/source-decisions/README.md](reference/source-decisions/README.md)
  for implemented decision rationale.
- Use [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md)
  for planned or incomplete design-wall rationale.

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
