# Proposals

This directory keeps proposed design targets that are not fully implemented.
Proposal text is not current language behavior unless
`../specification/` also states it.

## Read First

- Any proposal in this directory may be chosen for implementation work when it
  describes behavior absent from `../specification/`.
- Status labels: [../document-status.md](../document-status.md).

## Read When

- Use [implementation-route.md](implementation-route.md) for proposal promotion
  mechanics after an explicit task selects proposal work.
- Use [reference-followups.md](reference-followups.md) for follow-up work that
  is absent from the current specification.
- Use [self-hosting-standard-library.md](self-hosting-standard-library.md) only
  for future self-hosting standard library questions whose behavior is absent
  from the current specification.
- Use [jvm-bytecode-backend.md](jvm-bytecode-backend.md) for the proposal
  to replace Java source generation with direct JVM classfile emission,
  including runtime behavior, bytecode verification, and CI strategy.
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
