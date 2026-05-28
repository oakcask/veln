# Proposal Target Selection

Status: routing

Use this page when a task asks for the current target proposal, or when target
state is missing, stale, broad, exploratory, or points at implemented history.
This is the single proposal-target decision page and candidate-class index. It
does not define current language behavior or implementation steps.

## Read First

- Current decision: no active proposal target; the routing decision itself is
  complete for implementation prompts.
- Use the evidence and outcome table below before opening candidate pages.
- Continue to [implementation-route.md](implementation-route.md) only after one
  short proposal page names one absent behavior.
- Current implemented behavior stays in
  [../specification/README.md](../specification/README.md).

## Prompt Evidence

| Evidence | Decision |
| --- | --- |
| `prompts/TARGET.md` is absent. | Do not infer a target. |
| `prompts/NOTARGET` says no implementation target is selected from the current proposals. | Keep selection unset. |

## Selection Outcomes

Use this table instead of reopening candidate pages just to decide whether work
can proceed.

| Class | Decision | Next route |
| --- | --- | --- |
| No target | Stop before implementation, promotion, or specification updates; leave `../specification/` unchanged. | Stop here, or create one short proposal page before implementation work. |
| Active target | Continue only when one short proposal page names one absent behavior. | [implementation-route.md](implementation-route.md). |
| Implemented record | Treat as history or cleanup evidence; use the matching specification page for current behavior. | [formatter-stabilization.md](formatter-stabilization.md), [jvm-bytecode-backend.md](jvm-bytecode-backend.md), [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md). |
| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md). |
| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md). |
| Helper candidate pool | Choose exactly one descriptor-only pure helper, then create or select one short proposal page. | [self-hosting-standard-library.md](self-hosting-standard-library.md). |

## Selection Check

1. Read `prompts/TARGET.md` when it exists.
2. When `prompts/TARGET.md` is absent, read `prompts/NOTARGET` if present.
3. Keep selection unset when the prompt state says no target is selected.
4. If a target is selected, verify that it is a short proposal page, not a full detail
   record, review, reference note, broad index, helper candidate pool, or
   implemented proposal record.
5. If a target remains valid, compare the selected behavior with
   [../specification/README.md](../specification/README.md).
6. If the behavior is already implemented, use the matching specification page
   and treat the proposal as history or cleanup evidence.
7. If the behavior is broad, exploratory, or a helper candidate pool, split or
   create one short proposal before treating it as an implementation target.

Do not infer a target from broad follow-up indexes, exploratory inventories,
helper candidate pools, or implemented proposal records. Clarify routing or
create one short proposal page before treating proposal work as implementable.
For the no-target outcome, there is no proposal completion checklist; the
completion result is the routing decision without code, promotion, or
specification changes.

Evidence for the current no-target state lives in
[../reviews/no-proposal-target-completion.md](../reviews/no-proposal-target-completion.md).

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
