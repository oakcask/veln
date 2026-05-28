# Proposals

This directory keeps active proposal targets and implemented proposal records
that still route cleanup or promotion evidence. Proposal text is not current
language behavior unless `../specification/` also states it.

## Read First

- Current target and candidate classification:
  [target-selection.md](target-selection.md).
- Implementation starts only after target selection names one short proposal
  page for behavior absent from `../specification/`.
- Status labels: [../document-status.md](../document-status.md).

## Proposal Routes

- Target selection, no-target state, implemented records, broad indexes, and
  exploratory inventories: [target-selection.md](target-selection.md). Start
  here before inferring work from nearby proposal text.
- Proposal implementation after one target is selected:
  [implementation-route.md](implementation-route.md).
- Source-backed standard library:
  [self-hosting-standard-library.md](self-hosting-standard-library.md) records
  completed helper migrations and routes future helper selection.
- Broad follow-up index:
  [reference-followups.md](reference-followups.md).
- Design-wall inventory:
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md).
- Implemented proposal records:
  [formatter-stabilization.md](formatter-stabilization.md),
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md), and
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).

## Read When

- Use [target-selection.md](target-selection.md) when a target is missing,
  unset, stale, or too broad.
- Use [implementation-route.md](implementation-route.md) for proposal promotion
  mechanics after target selection names one concrete proposal.
- Use [formatter-stabilization.md](formatter-stabilization.md) for the
  implemented formatter stabilization record and completion evidence route.
- Use [jvm-bytecode-backend.md](jvm-bytecode-backend.md) for the implemented
  JVM backend proposal record. It routes current specification pages,
  completion evidence, and remaining cleanup without making backend layout
  details current specification.
- Use [reference-followups.md](reference-followups.md) for follow-up work that
  is absent from the current specification.
- Use [self-hosting-standard-library.md](self-hosting-standard-library.md)
  when checking completed prelude helper migrations or choosing the next
  descriptor-only pure-helper candidate.
- Use [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  for design-wall material that is exploratory, deferred, or already represented
  by reference decision records.
- Use [../reviews/README.md](../reviews/README.md) when checking gap evidence
  before changing target status.

## Update When

- A target is implemented and the resulting behavior has been documented under
  `../specification/`.
- A target is found to be already implemented by the current specification and
  only remaining proposal work should stay here.
- New proposal work is added, split, superseded, or removed.

## Skip Unless Needed

- Use `../specification/` when you need current implemented behavior.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
- Do not read implemented proposal records before the matching specification
  page unless you are checking history, evidence, or cleanup.
