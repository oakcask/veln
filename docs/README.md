# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them.

## Read First

- Current behavior: [reference/language/README.md](reference/language/README.md).
- Accepted or open targets: [proposals/target-queue.md](proposals/target-queue.md).
- Unclear route: [navigation.md](navigation.md).

## Choose One Task

- Implement behavior: start with
  [reference/language/topic-map.md](reference/language/topic-map.md), then use
  [proposals/implementation-route.md](proposals/implementation-route.md) only
  when a proposal must be promoted.
- Update diagnostics or command JSON:
  [reference/language/diagnostics-json.md](reference/language/diagnostics-json.md)
  or [reference/language/json-output.md](reference/language/json-output.md).
- Check rationale, review evidence, phase history, or source support:
  [navigation.md](navigation.md).
- Move or reclassify durable text: [document-status.md](document-status.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open a `*-full.md` file only when a short page names a section that matters.
- Return here instead of scanning sibling directories when the current route
  turns out to be proposal, review, phase, or reference work.

## Directory Map

- `reference/`: implemented behavior, durable rationale, and source support.
- `proposals/`: accepted or open targets that are not fully implemented.
- `reviews/`: evidence about gaps, verification, and completion claims.
- `phases/`: implementation order, working plans, and historical completion
  notes.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page identifies the
  section needed for the task.
- Do not read old phase plans before the current reference and review pages.
- Do not treat proposal text as implemented behavior unless the reference also
  states it.
