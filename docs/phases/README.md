# Implementation Phases

This directory keeps implementation-phase plans and working notes.

## Read First

- [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  records current gaps that must be fixed before treating the first-slice gate
  as complete.
- [first-slice-implementation.md](first-slice-implementation.md) describes the
  current first-slice architecture memo for the processor, standard library,
  runtime boundary, and implementation order.

## Skip Unless Needed

- Do not read the full implementation memo before the language reference when
  you only need current syntax or command behavior.
- Do not use a phase note as proof that behavior is implemented; check
  `../reference/language/` and `../reviews/` first.

## Read When

- Use this directory when starting or reviewing implementation work.
- Use `../reviews/` before relying on a phase completion claim.
- Use `../reference/source-decisions/` when you need implemented decision
  rationale.
- Use `../proposals/agent-language-spec-wall/` when you need planned or
  incomplete decision rationale.
- Use `../reference/` when a decision has been promoted to stable reference
  material.
