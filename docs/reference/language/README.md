# Language Specification

This directory contains the categorized specification for the implemented
first slice of Veln. It records the implemented subset, not every language
target tracked in `../../proposals/`.

## Read First

- [overview.md](overview.md): stability boundary and explicit non-goals.
- [source-surface.md](source-surface.md): implemented modules, items,
  expressions, tests, and grammar.
- [types.md](types.md): annotations, local inference, assignment, and operators.
- [commands.md](commands.md): source discovery, check, run, test, and format
  gates.

## Read When

- Names, stdio, prelude, and effects:
  [names-effects.md](names-effects.md).
- Contracts and holes: start with
  [contracts-holes.md](contracts-holes.md), then choose
  [contracts.md](contracts.md), [holes.md](holes.md), or
  [contracts-holes-full.md](contracts-holes-full.md).
- Machine-readable output: [diagnostics-json.md](diagnostics-json.md),
  [run-json.md](run-json.md), and [test-json.md](test-json.md).
- Runtime and examples: [execution.md](execution.md) and
  [examples.md](examples.md).
- Rationale: [source-decisions.md](source-decisions.md).

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  older proposal history.
- Use `../source-decisions/`, `../../proposals/`, `../../phases/`, or
  `../../reviews/` only after the current behavior page does not answer the
  question.
