# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them. Use [navigation.md](navigation.md) only when the
first route is not obvious.

## Read First

- Current language behavior:
  [reference/language/README.md](reference/language/README.md).
- If a proposal, phase note, or review conflicts with the language reference,
  treat the language reference as current behavior.
- Planned or accepted targets:
  [proposals/target-queue.md](proposals/target-queue.md).
- Stable reference map: [reference/README.md](reference/README.md).

## Choose One Task

- Change implemented language behavior:
  [reference/language/topic-map.md](reference/language/topic-map.md).
- Promote a proposal into implemented behavior:
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Decide whether proposal text can move into current behavior:
  [document-status.md](document-status.md).
- Update diagnostics, related notes, or command JSON behavior:
  [reference/language/diagnostics-json.md](reference/language/diagnostics-json.md)
  or [reference/language/json-output.md](reference/language/json-output.md).
- Check rationale behind current behavior:
  [reference/language/source-decisions.md](reference/language/source-decisions.md).
- Check review evidence, phase history, source support, or documentation
  maintenance routes: [navigation.md](navigation.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open `*-full.md` files and `result-*.md` records only when a short route
  names the relevant detail.
- Return here instead of scanning sibling directories when the route turns out
  to be proposal, review, phase, or reference work.

## Directory Map

- `reference/`: implemented behavior, durable rationale, and source support.
- `proposals/`: planned or accepted targets not fully implemented.
- `reviews/`: gap evidence, verification, and completion claims.
- `phases/`: implementation order, working plans, and historical notes.

## Skip Unless Needed

- Do not treat proposal text as implemented behavior unless the reference also
  states it.
- Do not read old phase plans before the current reference and review pages.
