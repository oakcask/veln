# Language Specification

This directory contains the categorized specification for the implemented
first slice of Veln. It records the implemented subset, not every language
target tracked in `../../proposals/`.

## Read First

- [overview.md](overview.md) defines the fixed stability boundary and the
  behavior that remains outside this reference.
- [source-surface.md](source-surface.md) defines implemented source syntax,
  expressions, and explicit non-goals.
- [types.md](types.md) defines annotations, local inference, assignment, and
  operators.
- [diagnostics-json.md](diagnostics-json.md) defines the stable
  `veln check --json` envelope and diagnostic detail fields.

## Read When

- [commands.md](commands.md): CLI behavior, source discovery, format gates,
  execution gates, and bootstrap test selection.
- [names-effects.md](names-effects.md): name resolution, stdio calls, prelude
  helpers, and public effect diagnostics.
- [contracts-holes.md](contracts-holes.md): implemented contract predicate
  checking, hole diagnostics, and `satisfy` constraints.
- [run-json.md](run-json.md): `veln run --json` schema and runtime error
  records.
- [test-json.md](test-json.md): `veln test --json` schema, case shape, and
  captured stdio events.
- [execution.md](execution.md): checked-core and backend boundaries.
- [examples.md](examples.md): complete examples that exercise implemented
  language behavior.
- [source-decisions.md](source-decisions.md): dated discussion results that
  support this specification.

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  older proposal history.
- Use `../source-decisions/` only for implemented rationale and decision
  history.
- Use `../../proposals/agent-language-spec-wall/` only for planned or
  incomplete decision history.
- Use `../../phases/` only for implementation plans and completion notes.
- Use `../../reviews/` only for current gaps and verification findings.
