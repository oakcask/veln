# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them.

## Read First

- Current behavior: [reference/language/README.md](reference/language/README.md).
- Accepted or open targets: [proposals/target-queue.md](proposals/target-queue.md).
- Unclear route: [navigation.md](navigation.md).
- Moving or reclassifying durable text: [document-status.md](document-status.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open a `*-full.md` file only when a short page names a section that matters.
- Return here instead of scanning sibling directories when the current route
  turns out to be proposal, review, phase, or reference work.

## Task Routes

- Current behavior work: use [reference/language/topic-map.md](reference/language/topic-map.md).
- Proposal implementation work: use
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Rationale or compatibility work: use
  [reference/language/source-decisions.md](reference/language/source-decisions.md).
- Research-source audit work: use
  [reference/bibliography/README.md](reference/bibliography/README.md).

## Directory Map

- `reference/`: implemented behavior, durable rationale, and source support.
- `proposals/`: accepted or open targets that are not fully implemented.
- `reviews/`: evidence about gaps, verification, and completion claims.
- `phases/`: implementation order, working plans, and historical completion
  notes.

## Route Boundaries

- A behavior page under `reference/language/` wins over proposal, phase, and
  review wording.
- A proposal page can describe an implementation target, but it is not current
  behavior until the language reference also says so.
- A review or phase page can explain why work happened, but it is not a route
  for changing the language specification.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page identifies the
  section needed for the task.
- Do not read old phase plans before the current reference and review pages.
- Do not treat proposal text as implemented behavior unless the reference also
  states it.
