# Formatter Stabilization

Status: proposed

This page owns the formatter follow-up target from
[reference-followups.md](reference-followups.md). It describes work that is not
current behavior unless `../specification/` also states it.

## Read First

- Current `veln fmt` behavior:
  [../specification/commands.md](../specification/commands.md), then
  [../specification/commands-full.md#veln-fmt-path](../specification/commands-full.md#veln-fmt-path)
  when exact command rules matter.
- Implemented comment and source syntax:
  [../specification/source-surface.md](../specification/source-surface.md).
- Promotion mechanics:
  [implementation-route.md](implementation-route.md).

## Current Boundary

The implemented formatter already has a whole-invocation parse gate,
deterministic formatting for implemented syntax, canonical tab indentation,
`match` arm indentation, standalone line-comment attachment to the next parsed
source line, trailing line-comment preservation, and multi-file idempotence
coverage. Treat those as current behavior through `../specification/`, not this
proposal page.

## Proposed Target

Define and implement formatter behavior beyond that current boundary, scoped to
comment attachment and formatting stabilization. Keep the work to formatter
behavior; do not include execution, test discovery, repair workflows, backend
replacement, or standard library expansion.

Before implementation, use this page only to route the target. After
implementation, promote the supported behavior into the smallest matching
`../specification/` page and leave only remaining absent formatter work here.

## Read When

- Choosing the formatter follow-up target from
  [reference-followups.md](reference-followups.md).
- Comparing proposed formatter behavior against the implemented `veln fmt`
  boundary before changing code or tests.
- Cleaning up proposal text after formatter behavior has been implemented and
  documented under `../specification/`.

## Skip Unless Needed

- Do not read this page for ordinary current `veln fmt` behavior.
- Do not use nearby follow-up bullets in
  [reference-followups.md](reference-followups.md) as formatter requirements.
- Do not move formatter behavior into `../specification/` until code and tests
  support it.
