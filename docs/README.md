# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, choose one route, and avoid opening full records
until a short page points to them.

## Read First

- Most tasks should open exactly one of the first two routes below, then stop
  when that route answers the question.
- [reference/language/README.md](reference/language/README.md): current
  implemented language behavior.
- [proposals/README.md](proposals/README.md): accepted or open implementation
  targets.
- [document-status.md](document-status.md): where durable text belongs when a
  proposal, review, or phase note changes status.
- [navigation.md](navigation.md): routes for rationale, reviews, phase history,
  and status work.

## Task Routes

- Current behavior work: use [reference/language/README.md](reference/language/README.md).
- Proposal implementation work: use [proposals/target-queue.md](proposals/target-queue.md).
- Rationale or compatibility work: use
  [reference/language/source-decisions.md](reference/language/source-decisions.md).
- Research-source audit work: use
  [reference/bibliography/README.md](reference/bibliography/README.md).
- Any other documentation route: use [navigation.md](navigation.md).

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
- Do not open bibliography records unless a source-decision or claim audit
  needs citation support.
