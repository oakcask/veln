# Proposals

This directory keeps proposal routes, target candidates, and implemented
proposal records that still carry useful history or cleanup evidence. Proposal
text is not current language behavior unless `../specification/` also states it.

## Read First

- Need the current target decision, prompt evidence, stale-target checks, or
  candidate classification:
  [target-selection.md](target-selection.md).
- Need implementation or promotion mechanics for an active target:
  [implementation-route.md](implementation-route.md). Open it only after
  target selection classifies one short proposal as active.
- Status labels: [../document-status.md](../document-status.md).

## Proposal Target Flow

1. Start with [target-selection.md](target-selection.md) for the target
   decision, prompt evidence, and candidate class.
2. Stop there when the outcome is no target, implemented record, broad index,
   exploratory inventory, or helper candidate pool.
3. Continue to [implementation-route.md](implementation-route.md) only for one
   active short proposal whose behavior is absent from `../specification/`.
4. Open candidate pages only after target selection names their class or next
   route:
   [reference-followups.md](reference-followups.md),
   [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md),
   [self-hosting-standard-library.md](self-hosting-standard-library.md),
   [formatter-stabilization.md](formatter-stabilization.md),
   [jvm-bytecode-backend.md](jvm-bytecode-backend.md), or
   [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).

## Read When

- Checking whether a proposal page can be the active target or must stay as
  history, inventory, or a candidate pool.
- Checking completed prelude helper migrations after target selection routes
  to that helper pool.
- Use [../reviews/README.md](../reviews/README.md) when checking gap evidence
  before changing target status.

## Update When

- A selected target becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A candidate's target class changes.
- New proposal work is added, split, superseded, or removed.

## Skip Unless Needed

- Use `../specification/` when you need current implemented behavior.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
- Do not read implemented proposal records before the matching specification
  page unless you are checking history, evidence, or cleanup.
