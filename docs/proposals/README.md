# Proposals

This directory keeps proposal routes, target candidates, and implemented
proposal records that still carry useful history or cleanup evidence. Proposal
text is not current language behavior unless `../specification/` also states it.

## Read First

- Current target, prompt evidence, and candidate classes:
  [target-selection.md](target-selection.md). It is the canonical route for
  missing, stale, broad, exploratory, implemented-history, helper-pool, and
  unset target state.
- Implementation route after one active short proposal is selected:
  [implementation-route.md](implementation-route.md).
- Status labels: [../document-status.md](../document-status.md).

## Routes

- Target decision:
  [target-selection.md](target-selection.md).
- Implementation after target selection names one active short page:
  [implementation-route.md](implementation-route.md).
- Candidate details only after target selection routes there:
  [reference-followups.md](reference-followups.md),
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md),
  [self-hosting-standard-library.md](self-hosting-standard-library.md),
  [formatter-stabilization.md](formatter-stabilization.md),
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md), or
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).

## Read When

- Checking whether a proposal page can be the active target.
- Choosing one implementable proposal before any promotion route starts.
- Checking completed prelude helper migrations only after target selection
  routes to that helper pool.
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
