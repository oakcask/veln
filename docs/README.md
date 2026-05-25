# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reference/language/README.md](reference/language/README.md): current
  implemented language behavior.
- [proposals/README.md](proposals/README.md): accepted or open implementation
  targets.
- [document-status.md](document-status.md): placement rules when moving text
  between proposal, review, phase, and reference areas.

## Routes

- Changing syntax, types, effects, contracts, holes, commands, JSON output, or
  examples: start at [reference/language/README.md](reference/language/README.md).
- Planning the next slice: start at [proposals/README.md](proposals/README.md),
  then check [reviews/README.md](reviews/README.md) for evidence and
  [document-status.md](document-status.md) for promotion rules.
- Explaining why implemented behavior exists: use
  [reference/language/source-decisions.md](reference/language/source-decisions.md),
  then open [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Reconstructing implementation order: use
  [phases/README.md](phases/README.md).
- Reading incomplete design-wall rationale: use
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).

## Skip Unless Needed

- Do not read old phase plans before the current reference and review pages.
- Do not read source-decision records when the language reference answers the
  behavior question.
- Do not treat proposal text as implemented behavior unless the reference also
  states it.
