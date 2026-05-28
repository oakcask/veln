# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them. Use [navigation.md](navigation.md) only when the
first route is not obvious.

## Read First

- Current language behavior:
  [specification/README.md](specification/README.md).
- If a proposal or review conflicts with the language specification,
  treat the specification as current behavior.
- Planned or accepted proposal work:
  [proposals/README.md](proposals/README.md).
- Proposal target decision:
  [proposals/target-selection.md](proposals/target-selection.md).
- Rationale and source-support map: [reference/README.md](reference/README.md).

## Choose One Task

- Change implemented language behavior:
  [specification/topic-map.md](specification/topic-map.md).
- Find, confirm, or reject a proposal target:
  [proposals/target-selection.md](proposals/target-selection.md).
- Promote a selected proposal into implemented behavior:
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Decide whether proposal text can move into current behavior:
  [document-status.md](document-status.md).
- Update diagnostics, related notes, or command JSON behavior:
  [specification/diagnostics-json.md](specification/diagnostics-json.md)
  or [specification/json-output.md](specification/json-output.md).
- Check rationale behind current behavior:
  [specification/source-decisions.md](specification/source-decisions.md).
- Check review evidence, source support, or documentation
  maintenance routes: [navigation.md](navigation.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open `*-full.md` files and `result-*.md` records only when a short route
  names the relevant detail.
- Return here instead of scanning sibling directories when the route turns out
  to be proposal, review, or reference work.

## Directory Map

- `specification/`: current implemented language behavior, kept as the latest
  specification only.
- `reference/`: durable rationale and source support.
- `proposals/`: planned or accepted targets not fully implemented.
- `reviews/`: gap evidence, verification, and completion claims.

## Skip Unless Needed

- Do not treat proposal text as implemented behavior unless the specification
  also states it.
- Do not read old review records before the current specification page or the
  target-selection route answers the task.
