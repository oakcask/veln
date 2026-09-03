---
role: routing
update-when: A documentation route is added, moved, reclassified, or no longer answers the routed task.
---

# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and stop at the smallest page that
answers the task. Use [navigation.md](navigation.md) only when the first route
is not obvious.

## Read First

- Current language behavior:
  [specification/README.md](specification/README.md).
- Planned or accepted proposal work:
  [proposals/README.md](proposals/README.md).
- Rationale, source support, and implemented proposal records:
  [reference/README.md](reference/README.md).

## Choose One Task

- Change implemented language behavior:
  [specification/topic-map.md](specification/topic-map.md).
- Promote proposal work into implemented behavior:
  [proposals/README.md](proposals/README.md), then the matching
  specification page.
- Update diagnostics, related notes, or command JSON behavior:
  [specification/diagnostics-json.md](specification/diagnostics-json.md)
  or [specification/json-output.md](specification/json-output.md).
- Update MCP workspace selection, saved diagnostics, saved navigation,
  language-reference resources, or tool schemas:
  [specification/mcp.md](specification/mcp.md).
- Check rationale behind current behavior:
  [specification/source-decisions.md](specification/source-decisions.md).
- Author or maintain documentation:
  [reference/documentation-authoring.md](reference/documentation-authoring.md),
  then the README for the affected directory.
- Check a documentation route that is not listed here:
  [navigation.md](navigation.md).

## Stop Rule

- Stop at the first short page that answers the task.
- Open detail or `result-*.md` records only when a route names the relevant
  subject.
- Return here instead of scanning sibling directories when the route turns out
  to be proposal or reference work.

## Directory Map

- `specification/`: current implemented language behavior, kept as the latest
  specification only.
- `reference/`: durable policy, rationale, source support, and completed
  proposal records.
- `proposals/`: planned or accepted targets not fully implemented. Proposal
  pages in this directory declare `role: proposal`.

## Skip Unless Needed

- Use the directory README files for status and placement routes instead of
  repeating those rules here.
- Do not read implemented proposal records before the current specification page
  and [reference/implemented-proposals/README.md](reference/implemented-proposals/README.md)
  route.
