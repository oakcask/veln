# Formatter Stabilization

Status: implemented

This page routes the implemented formatter follow-up target from
[reference-followups.md](reference-followups.md). Use the specification pages
for current `veln fmt` behavior.

## Read First

- Current formatter behavior:
  [../specification/commands.md](../specification/commands.md), then
  [../specification/commands-full.md#veln-fmt-path](../specification/commands-full.md#veln-fmt-path)
  when exact command rules matter.
- Implemented comment and source syntax:
  [../specification/source-surface.md](../specification/source-surface.md).
- Completion evidence:
  [../reviews/formatter-stabilization-completion.md](../reviews/formatter-stabilization-completion.md).
- Promotion mechanics:
  [implementation-route.md](implementation-route.md).

## Outcome

The selected target promoted formatter comment attachment and formatting
stabilization into `../specification/`. Standalone line comments now attach to
the next parsed module header, import, function signature, contract clause,
body line, or closing `end` line. Trailing line comments remain on the same
formatted source line.

This proposal page is now history and routing. New formatter work should use a
new proposal page unless it is already stated by `../specification/`.

## Read When

- Checking why the formatter follow-up target is no longer listed as active.
- Reviewing completion evidence before removing or superseding this route.
- Auditing proposal promotion mechanics for formatter behavior.

## Skip Unless Needed

- Do not read this page for ordinary current `veln fmt` behavior.
- Do not use this page as a source of current command behavior.
- Do not use nearby follow-up bullets in
  [reference-followups.md](reference-followups.md) as formatter requirements.
