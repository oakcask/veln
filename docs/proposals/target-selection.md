# Proposal Target Selection

Status: routing

Use this page when a task asks for the current target proposal, or when target
state is missing, stale, broad, exploratory, or points at implemented history.
This page owns proposal-target classification only; it does not define current
language behavior.

## Read First

- Current prompt state has no active proposal target. Stop here before
  implementation or promotion work unless a later prompt selects one concrete
  short proposal page.
- Current implemented behavior stays in
  [../specification/README.md](../specification/README.md).
- An active implementation target must be one short proposal page that names one
  absent behavior.
- Use [implementation-route.md](implementation-route.md) only after that target
  exists.

## Current State

- `prompts/TARGET.md` is absent.
- `prompts/NOTARGET` says no implementation target is selected from the
  current proposals.
- Result: no active proposal target.

## Target Classes

Use this table instead of reopening candidate pages just to decide whether a
target exists.

| Class | Rule | Route |
| --- | --- | --- |
| No target | Keep selection unset. | Stop here or create one short proposal page. |
| Active target | Continue only when one short proposal page names one absent behavior. | [implementation-route.md](implementation-route.md). |
| Implemented record | Treat as history or cleanup evidence; use the matching specification page for current behavior. | [formatter-stabilization.md](formatter-stabilization.md), [jvm-bytecode-backend.md](jvm-bytecode-backend.md), [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md). |
| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md). |
| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md). |
| Helper candidate pool | Choose exactly one descriptor-only pure helper, then create or select one short proposal page. | [self-hosting-standard-library.md](self-hosting-standard-library.md). |

## Selection Algorithm

1. Read `prompts/TARGET.md` when it exists.
2. When `prompts/TARGET.md` is absent, read `prompts/NOTARGET` if present.
3. Keep selection unset when the prompt state says no target is selected.
4. Verify that any selected page is a short proposal page, not a full detail
   record, review, reference note, broad index, helper candidate pool, or
   implemented proposal record.
5. Compare the selected behavior with
   [../specification/README.md](../specification/README.md).
6. If the behavior is already implemented, use the matching specification page
   and treat the proposal as history or cleanup evidence.
7. If the behavior is broad, exploratory, or a helper candidate pool, split or
   create one short proposal before treating it as an implementation target.

## Handoff

- With the current prompt state, there is no proposal completion checklist to
  promote into `../specification/`.
- Leave current behavior unchanged and keep `../specification/` untouched.
- The next implementation pass should first create or select one short proposal
  page whose behavior is absent from `../specification/`, then use
  [implementation-route.md](implementation-route.md).

## Read When

- A prompt target is missing, stale, or points to broad proposal material.
- Choosing whether a proposal page is active work or implemented history.
- Splitting broad follow-up material into one implementable proposal.
- Auditing that proposal text stays out of current behavior documentation until
  code and tests support it.

## Skip Unless Needed

- Do not use this page for current source syntax, command behavior, helper
  semantics, diagnostics, or runtime behavior.
- Do not open full proposal records until a short proposal page names the
  specific detail needed.
