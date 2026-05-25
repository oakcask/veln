# Language Specification

This directory contains the categorized specification for the implemented
first slice of Veln. It records the implemented subset, not every language
target tracked in `../../proposals/`.

## Read First

- [overview.md](overview.md): stability boundary and explicit non-goals.
- [source-surface.md](source-surface.md): implemented modules, items,
  expressions, tests, and grammar.

## Update When

- A proposal becomes implemented behavior.
- A test, diagnostic, command, JSON schema, or example changes the observable
  language surface.
- A source decision is promoted from planned rationale to implemented behavior.

Keep this directory focused on behavior supported by current code and tests.
Leave rationale in [source-decisions.md](source-decisions.md) or
`../source-decisions/` unless it changes how users should read the language.

## Read When

- Types, annotations, inference, assignment, and operators:
  [types.md](types.md), then [types-full.md](types-full.md) for detail.
- Commands, source discovery, check, run, test, and format gates:
  [commands.md](commands.md).
- Names, stdio, prelude, and effects:
  [names-effects.md](names-effects.md).
- Contracts and holes: start with
  [contracts-holes.md](contracts-holes.md), then choose
  [contracts.md](contracts.md), [holes.md](holes.md), or
  [contracts-holes-full.md](contracts-holes-full.md).
- Machine-readable output: start with [json-output.md](json-output.md), then
  choose [diagnostics-json.md](diagnostics-json.md),
  [run-json.md](run-json.md), or [test-json.md](test-json.md).
- Runtime and examples: [execution.md](execution.md), then
  [execution-full.md](execution-full.md) for detail, and
  [examples.md](examples.md).
- Rationale: [source-decisions.md](source-decisions.md).

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  older proposal history.
- Use [overview.md](overview.md) only when you need the stability boundary or
  explicit non-goals.
- Use `*-full.md` files only after their short routing page identifies the
  relevant section.
- Use `../source-decisions/`, `../../proposals/`, `../../phases/`, or
  `../../reviews/` only after the current behavior page does not answer the
  question.
