---
role: implementation-record
authority: supporting
update-when: Formatter command behavior, formatter source syntax, or formatter executable examples change.
---

# Formatter Stabilization

This page routes the implemented formatter follow-up target. Use the
specification pages for current `veln fmt` behavior.

## Read First

- Current formatter behavior:
  [../../specification/commands.md](../../specification/commands.md), then
  [../../specification/command-fmt.md](../../specification/command-fmt.md)
  when exact command rules matter.
- Implemented comment and source syntax:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Completion evidence is summarized in this page.

## Outcome

The selected target promoted formatter comment attachment and formatting
stabilization into `../../specification/`. Standalone line comments now attach to
the next parsed module header, import, function signature, contract clause,
body line, or closing `end` line. Trailing line comments remain on the same
formatted source line.

This proposal page is now history and routing. New formatter work should use a
new proposal page unless it is already stated by `../../specification/`.

Parser recovery around formatter-owned layout accepts comment-separated imports
and contract clauses through the same newline-tolerant declaration paths used
by ordinary source parsing. Completion coverage included formatter tree tests
for imports, contracts, and `end` lines, plus CLI JSON coverage for the same
comment attachment behavior.

## Read When

- Checking why the formatter follow-up target is no longer listed as active.
- Reviewing completion evidence before removing or superseding this route.
- Auditing proposal promotion mechanics for formatter behavior.

## Skip Unless Needed

- Do not read this page for ordinary current `veln fmt` behavior.
- Do not use this page as a source of current command behavior.
- Do not use removed follow-up inventories as formatter requirements.
