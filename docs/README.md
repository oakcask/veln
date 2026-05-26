# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them.

## Read First

- Stable reference map: [reference/README.md](reference/README.md).
- Current language behavior: [reference/language/README.md](reference/language/README.md).
- Planned or accepted targets: [proposals/target-queue.md](proposals/target-queue.md).
- Route not obvious: [navigation.md](navigation.md).

## Choose One Task

- Change implemented behavior:
  [reference/language/topic-map.md](reference/language/topic-map.md).
- Promote a proposal into implemented behavior:
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Update diagnostics, related notes, or command JSON:
  [reference/language/diagnostics-json.md](reference/language/diagnostics-json.md)
  or [reference/language/json-output.md](reference/language/json-output.md).
- Check rationale or source support:
  [reference/language/source-decisions.md](reference/language/source-decisions.md).
- Check review evidence or phase history: [navigation.md](navigation.md).
- Move or reclassify durable text: [document-status.md](document-status.md).
- Maintain entry points, routing pages, or link health:
  [navigation.md#documentation-maintenance](navigation.md#documentation-maintenance).

## Audit Routes

- Source-decision record placement:
  [reference/source-decisions/result-index.md](reference/source-decisions/result-index.md).
- Exhaustive source-decision storage:
  [reference/source-decisions/records/README.md](reference/source-decisions/records/README.md).
- Broad documentation routing rules: [navigation.md](navigation.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open a `*-full.md` file only when a short page names a section that matters.
- Open `result-*.md` source-decision records only when a category route or
  audit route names the relevant record.
- Return here instead of scanning sibling directories when the current route
  turns out to be proposal, review, phase, or reference work.

## Directory Map

- `reference/`: implemented behavior, durable rationale, and source support.
- `proposals/`: planned or accepted targets not fully implemented.
- `reviews/`: gap evidence, verification, and completion claims.
- `phases/`: implementation order, working plans, and historical notes.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page identifies the
  section needed for the task.
- Do not read old phase plans before the current reference and review pages.
- Do not treat proposal text as implemented behavior unless the reference also
  states it.
