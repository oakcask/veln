# Proposals

This directory keeps active proposal targets and implemented proposal records
that still route cleanup or promotion evidence. Proposal text is not current
language behavior unless `../specification/` also states it.

## Read First

- Choose an active proposal for implementation work only when it describes
  behavior absent from `../specification/`.
- Treat implemented proposal records as history and cleanup routes. Use the
  matching specification page for current behavior.
- Status labels: [../document-status.md](../document-status.md).

## Proposal Routes

- Source-backed standard library:
  [self-hosting-standard-library.md](self-hosting-standard-library.md) records
  completed `vec_map`, `vec_try_map`, and `vec_try_map_with` migrations, then
  routes future candidates back through the implemented helper split.

## Read When

- Use [implementation-route.md](implementation-route.md) for proposal promotion
  mechanics after an explicit task selects proposal work.
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
  for design-wall material that is still exploratory or only partially
  represented.
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
