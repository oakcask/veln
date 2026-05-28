# Proposal Target Selection

Status: routing

Use this page when a task asks for the current target proposal, when prompt
state is missing or stale, or when proposal candidates look similar. This page
routes selection only; it does not define current language behavior.

## Current State

- Current target: none.
- Prompt evidence: `prompts/TARGET.md` is absent, and `prompts/NOTARGET` says
  no implementation target is selected from the current proposals.
- Decision: keep the target unset instead of inferring work from nearby
  proposal text.

## Read First

- Current implemented behavior stays in
  [../specification/README.md](../specification/README.md).
- An implementation target must be one short proposal page that names one absent
  behavior.
- If no short proposal page meets that rule, stop here or create one before any
  implementation route starts.

## Target Classes

| Class | How to proceed | Current examples |
| --- | --- | --- |
| No target | Keep selection unset. Stop here or create one short proposal page. | Current prompt state |
| Active target | Use [implementation-route.md](implementation-route.md). | None |
| Implemented proposal record | Use the matching specification page for current behavior. Open the proposal only for history, evidence, or cleanup. | [formatter-stabilization.md](formatter-stabilization.md), [jvm-bytecode-backend.md](jvm-bytecode-backend.md), [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md) |
| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md) |
| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md) |
| Helper candidate pool | Choose exactly one descriptor-only pure helper, then create or select one short proposal page. | [self-hosting-standard-library.md](self-hosting-standard-library.md) |

## Selection Rule

1. Read `prompts/TARGET.md` when it exists.
2. When `prompts/TARGET.md` is absent, read `prompts/NOTARGET` if present and
   keep the target unset when it says no implementation target is selected.
3. Verify that the selected page is a short proposal page, not a full detail
   record, review, reference note, or implemented proposal record.
4. Compare the selected behavior with
   [../specification/README.md](../specification/README.md).
5. If the behavior is already implemented, use the matching specification page
   and treat the proposal as history or cleanup evidence.
6. If the behavior is broad, exploratory, or a helper candidate pool, split or
   create one short proposal before treating it as an implementation target.

## Handoff

The current prompt state has no selected target, so there is no proposal
completion checklist to promote into `../specification/`. Do not update the
specification or create a stop marker from this state alone.

The next implementation pass should first create or select one short proposal
page whose behavior is absent from `../specification/`, then use
[implementation-route.md](implementation-route.md) for the comparison and
promotion route.

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
