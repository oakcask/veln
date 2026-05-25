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
- Use [reference/language/source-surface.md](reference/language/source-surface.md)
  for implemented syntax, expression, and source grammar questions.
- Use [reference/language/diagnostics-json.md](reference/language/diagnostics-json.md),
  [reference/language/run-json.md](reference/language/run-json.md), or
  [reference/language/test-json.md](reference/language/test-json.md) before
  changing human or machine-readable command output.
- Use [phases/README.md](phases/README.md) for implementation order and working
  plans.
- Use [reference/source-decisions/README.md](reference/source-decisions/README.md)
  for implemented decision rationale.
- Use [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md)
  for planned or incomplete design-wall rationale.

## Current Shape

- `reference/language/` is the first stop for implemented behavior.
- `reference/source-decisions/` is historical rationale grouped by topic.
- `phases/` and `reviews/` contain longer working records; read their README
  files before opening the long detail documents.
- `proposals/` contains accepted or open targets that still need promotion into
  the reference after implementation.

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
