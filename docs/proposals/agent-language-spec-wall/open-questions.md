# Agent-Oriented Language Spec Wall Open Questions

Status: proposed

This page routes the unresolved question set and resolved decision pointers.
Resolved decision bodies live under `../../reference/source-decisions/`; the
full question inventory is kept in [open-questions-full.md](open-questions-full.md).

## Current State

- The full inventory currently routes resolved first-slice questions to
  decision records and current specification pages.
- Treat new design-wall work as proposal work before changing current
  behavior.

## Read First

- Proposed repair command target:
  [repair-command.md](repair-command.md).
- The first-slice implementation questions are already resolved and moved to
  `../../reference/source-decisions/`.
- Current implementation behavior should be read from
  `../../specification/`, not from this proposal inventory.
- Use [open-questions-full.md](open-questions-full.md) only when auditing old
  design-wall coverage or moving another resolved item.

## Read When

- Implementation readiness, parser, checker, runtime, and diagnostics:
  [open-questions-full.md#implementation-readiness](open-questions-full.md#implementation-readiness).
- Repair-loop command boundary, candidate schema, edit representation, ranking,
  and confirmation protocol: [repair-command.md](repair-command.md).
- Surface syntax, types, runtime, contracts, effects, holes, toolchain, and
  module documentation topics:
  [open-questions-full.md#surface-syntax](open-questions-full.md#surface-syntax).
- Broad design thesis and target shape: [design-brief.md](design-brief.md).

## Skip Unless Needed

- Do not read the full question inventory before the language specification when
  checking implemented behavior.
- Do not edit this page as a decision record; add or move durable decisions
  under `../../reference/source-decisions/`.
