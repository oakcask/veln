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

## Task Routes

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

## Read When

- Use this directory before changing code, tests, diagnostics, or samples.
- Use the task route above before opening proposals, phase notes, or source
  decisions.

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  older proposal history.
- Use `../source-decisions/` only for implemented rationale and decision
  history.
- Use `../../proposals/agent-language-spec-wall/` only for planned or
  incomplete decision history.
- Use `../../phases/` only for implementation plans and completion notes.
- Use `../../reviews/` only for current gaps and verification findings.
