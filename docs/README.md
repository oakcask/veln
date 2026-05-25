# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- Start with the first route below that matches the task. Open long detail
  files only after a route page points to the needed section.
- [reference/language/README.md](reference/language/README.md): current
  implemented language behavior.
- [reference/language/topic-map.md](reference/language/topic-map.md): fastest
  route from task area to the relevant language reference.
- [proposals/README.md](proposals/README.md): accepted or open implementation
  targets.
- [proposals/target-queue.md](proposals/target-queue.md): shortest route for
  selecting one accepted proposal to implement.
- [document-status.md](document-status.md): placement rules when moving text
  between proposal, review, phase, and reference areas.

## Routes

- Changing syntax, types, effects, contracts, holes, commands, JSON output, or
  examples: start at [reference/language/topic-map.md](reference/language/topic-map.md).
- Choosing an implementation target: start at
  [proposals/target-queue.md](proposals/target-queue.md), then use
  [proposals/implementation-route.md](proposals/implementation-route.md) for the
  proposal-to-reference workflow.
- Checking whether a target is still justified: use
  [reviews/README.md](reviews/README.md) for evidence and
  [document-status.md](document-status.md) for promotion rules.
- Explaining why implemented behavior exists: use
  [reference/language/source-decisions.md](reference/language/source-decisions.md),
  then open [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Reconstructing implementation order: use
  [phases/README.md](phases/README.md).
- Reading incomplete design-wall rationale: use
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).

## Directory Roles

- `reference/`: implemented behavior, durable rationale, and source families.
- `proposals/`: accepted or open targets that still need implementation or
  promotion into the reference.
- `reviews/`: evidence about gaps, verification, and completion claims.
- `phases/`: implementation order, working plans, and historical completion
  notes.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page identifies the
  section needed for the task.
- Do not read old phase plans before the current reference and review pages.
- Do not read source-decision records when the language reference answers the
  behavior question.
- Do not treat proposal text as implemented behavior unless the reference also
  states it.
