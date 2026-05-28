# Proposals

This directory keeps proposal routes, active target candidates, and implemented
proposal records that still route useful history or cleanup evidence. Proposal
text is not current language behavior unless `../specification/` also states it.

## Read First

- Current target, prompt evidence, and candidate classification:
  [target-selection.md](target-selection.md).
- Stop at target selection when it says no target is active.
- Open [implementation-route.md](implementation-route.md) only after target
  selection names one active short proposal.
- Status labels: [../document-status.md](../document-status.md).

## Choose One Route

- Missing, stale, broad, exploratory, or unset target:
  [target-selection.md](target-selection.md).
- Implementation after one active short target is selected:
  [implementation-route.md](implementation-route.md).
- Source-backed standard library helper selection:
  [self-hosting-standard-library.md](self-hosting-standard-library.md).
- Broad follow-up ideas that need short target pages:
  [reference-followups.md](reference-followups.md).
- Agent-language design-wall inventory:
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md).
- Implemented proposal records:
  [formatter-stabilization.md](formatter-stabilization.md),
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md), and
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).

## Read When

- Use [target-selection.md](target-selection.md) when a target is missing,
  unset, stale, too broad, or appears to point at implemented history.
- Use [implementation-route.md](implementation-route.md) for proposal promotion
  mechanics after target selection names one concrete proposal.
- Use [reference-followups.md](reference-followups.md) for follow-up work that
  is absent from the current specification.
- Use [self-hosting-standard-library.md](self-hosting-standard-library.md)
  when checking completed prelude helper migrations or choosing the next
  descriptor-only pure-helper candidate.
- Use [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  for design-wall inventory after target selection says the directory itself is
  not an active target.
- Use [../reviews/README.md](../reviews/README.md) when checking gap evidence
  before changing target status.

## Update When

- A selected target becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A candidate is found to be implemented history, broad follow-up work, or
  exploratory inventory.
- New proposal work is added, split, superseded, or removed.

## Skip Unless Needed

- Use `../specification/` when you need current implemented behavior.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
- Do not read implemented proposal records before the matching specification
  page unless you are checking history, evidence, or cleanup.
