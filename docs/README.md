# Veln Design Notes

This directory routes design notes, implemented language behavior, proposals,
reviews, and phase history for the experimental Veln implementation.

## Read First

- [reference/language/README.md](reference/language/README.md): current
  implemented language behavior.
- [proposals/README.md](proposals/README.md): accepted or open implementation
  targets.
- [document-status.md](document-status.md): where durable text belongs when a
  proposal, review, or phase note changes status.

## Task Routes

- Finding the smallest current behavior page:
  [reference/language/topic-map.md](reference/language/topic-map.md).
- Changing syntax, types, effects, contracts, holes, commands, JSON output,
  runtime behavior, or examples:
  [reference/language/topic-map.md](reference/language/topic-map.md).
- Choosing an implementation target: start at
  [proposals/target-queue.md](proposals/target-queue.md), then use
  [proposals/implementation-route.md](proposals/implementation-route.md) for the
  proposal-to-reference workflow.
- Checking whether proposal text is current behavior: start with
  [reference/language/README.md](reference/language/README.md), then compare
  the selected proposal through
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Moving text between proposal, review, phase, and reference areas:
  [document-status.md](document-status.md).
- Checking gap evidence or completion claims:
  [reviews/README.md](reviews/README.md).
- Reconstructing implementation order: [phases/README.md](phases/README.md).
- Explaining why implemented behavior exists:
  [reference/language/source-decisions.md](reference/language/source-decisions.md),
  then [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Reading incomplete design-wall rationale:
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).

## Reading Order

- Current behavior work: `reference/language/README.md` first, then the
  topic-specific short page, then the matching `*-full.md` file only if named.
- Implementation target work: `proposals/target-queue.md` first,
  `proposals/implementation-route.md` second, then the selected proposal route
  and current reference pages for comparison.
- Rationale work: current reference page first, then
  `reference/language/source-decisions.md`, then one source-decision category.
- Status or movement work: `document-status.md` before editing labels or moving
  text between directories.

## Directory Map

- `reference/`: implemented behavior, durable rationale, and source families.
- `proposals/`: accepted or open targets that still need implementation.
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
