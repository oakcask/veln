# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them. Use [navigation.md](navigation.md) only when the
first route is not obvious.

## Read First

- Current language behavior:
  [specification/README.md](specification/README.md).
- Planned or accepted proposal work:
  [proposals/README.md](proposals/README.md).
- Rationale and source-support map: [reference/README.md](reference/README.md).
- Documentation placement and promotion rules:
  [document-status.md](document-status.md).

## Choose One Task

- Change implemented language behavior:
  [specification/topic-map.md](specification/topic-map.md).
- Promote proposal work into implemented behavior:
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Update diagnostics, related notes, or command JSON behavior:
  [specification/diagnostics-json.md](specification/diagnostics-json.md)
  or [specification/json-output.md](specification/json-output.md).
- Check rationale behind current behavior:
  [specification/source-decisions.md](specification/source-decisions.md).
- Check source support or documentation maintenance routes:
  [navigation.md](navigation.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open `*-full.md` files and `result-*.md` records only when a short route
  names the relevant detail.
- Return here instead of scanning sibling directories when the route turns out
  to be proposal or reference work.

## Directory Map

- `specification/`: current implemented language behavior, kept as the latest
  specification only.
- `reference/`: durable rationale and source support.
- `proposals/`: planned or accepted targets not fully implemented.

## Skip Unless Needed

- Use [document-status.md](document-status.md) for status and placement rules
  instead of repeating those rules here.
- Do not read implemented proposal records before the current specification page
  or the proposal route answers the task.
