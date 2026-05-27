# Agent-Oriented Language Spec Wall Open Questions

Status: open-proposal

This page routes the unresolved question set and resolved decision pointers.
Resolved decision bodies live under `../../reference/source-decisions/`; the
full question inventory is kept in [open-questions-full.md](open-questions-full.md).

## Current State

- The full inventory currently routes resolved first-slice questions to
  decision records and current reference pages.
- No accepted follow-up target remains in
  [../first-slice-follow-ups.md](../first-slice-follow-ups.md) or
  [../target-queue.md](../target-queue.md).
- Treat any new design-wall work as proposal selection work before changing
  current behavior.

## Read First

- The first-slice implementation questions are already resolved and promoted
  to `../../reference/source-decisions/`.
- Current implementation behavior should be read from
  `../../reference/language/`, not from this proposal inventory.
- Use [open-questions-full.md](open-questions-full.md) only when auditing old
  design-wall coverage or promoting another resolved item.

## Read When

- Implementation readiness, parser, checker, runtime, and diagnostics:
  [open-questions-full.md#implementation-readiness](open-questions-full.md#implementation-readiness).
- Surface syntax, types, runtime, contracts, effects, holes, toolchain, and
  module documentation topics:
  [open-questions-full.md#surface-syntax](open-questions-full.md#surface-syntax).
- Broad design thesis and target shape: [design-brief.md](design-brief.md).

## Skip Unless Needed

- Do not read the full question inventory before the language reference when
  checking implemented behavior.
- Do not edit this page as a decision record; add or move durable decisions
  under `../../reference/source-decisions/`.
