# Proposal Target Selection

Status: routing

Use this page when a task asks for the current target proposal, when
`prompts/TARGET.md` does not select one concrete proposal, or when proposal
candidates look similar. This page routes selection only; it does not define
current language behavior.

## Read First

- Current target: none. `prompts/TARGET.md` is absent, and `prompts/NOTARGET`
  says no implementation target is selected from the current proposals.
- Current implemented behavior stays in
  [../specification/README.md](../specification/README.md).
- An implementation target must be one short proposal page that names one
  missing behavior absent from the current specification.
- Implemented proposal records, broad follow-up indexes, and exploratory design
  inventories are not active targets by themselves.
- If `prompts/TARGET.md` is absent and `prompts/NOTARGET` is present, keep the
  target unset instead of inferring one from nearby proposal text.

## Selection Rule

1. Read `prompts/TARGET.md` when it exists.
2. Verify that the selected page is a short proposal page, not a full detail
   record, review, reference note, or implemented proposal record.
3. Compare the selected behavior with
   [../specification/README.md](../specification/README.md).
4. If the behavior is already implemented, use the matching specification page
   and treat the proposal as history or cleanup evidence.
5. If the behavior is broad or exploratory, split or create a short proposal
   before treating it as an implementation target.

## Candidate Map

- [self-hosting-standard-library.md](self-hosting-standard-library.md) has no
  active helper target. It can produce a future target only after choosing one
  descriptor-only pure helper from the full candidate list.
- [reference-followups.md](reference-followups.md) is a broad follow-up index.
  A listed area needs a short proposal page before it becomes an implementation
  target.
- [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  keeps exploratory design-wall material. Broader repair or design-wall work
  needs a new short proposal before it becomes a target.
- [formatter-stabilization.md](formatter-stabilization.md) and
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md), plus
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md),
  are implemented proposal records. Start from the matching specification page
  unless checking history, evidence, or cleanup.

## Read When

- A prompt target is missing, stale, or points to broad proposal material.
- Choosing whether a proposal page is active work or implemented history.
- Splitting broad follow-up material into one implementable proposal.
- Auditing that proposal text stays out of current behavior documentation until
  code and tests support it.

## Next Route

- After one concrete target is selected, use
  [implementation-route.md](implementation-route.md) for comparison and
  promotion mechanics.
- When no concrete target is selected, stop here or create a short proposal
  page before implementation work.

## Skip Unless Needed

- Do not use this page for current source syntax, command behavior, helper
  semantics, diagnostics, or runtime behavior.
- Do not open full proposal records until a short proposal page names the
  specific detail needed.
