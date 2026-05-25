# Language Specification

This directory contains the categorized specification for the implemented
first slice of Veln. It records the implemented subset, not every language
target tracked in `../../proposals/`.

## Read First

- [overview.md](overview.md): stability boundary and explicit non-goals.
- [topic-map.md](topic-map.md): choose the smallest topic page for a source,
  command, JSON, runtime, contract, or hole change.
- Use the short topic pages below first; open `*-full.md` only after the short
  page names the relevant detail.

## Update When

- A proposal becomes implemented behavior.
- A test, diagnostic, command, JSON schema, or example changes the observable
  language surface.
- A source decision is promoted from planned rationale to implemented behavior.

Keep this directory focused on behavior supported by current code and tests.
Leave rationale in [source-decisions.md](source-decisions.md) or
`../source-decisions/` unless it changes how users should read the language.

## Read When

- Task-oriented topic selection: [topic-map.md](topic-map.md).
- Source syntax and grammar details: [source-surface.md](source-surface.md),
  [types.md](types.md), and [names-effects.md](names-effects.md).
- Contracts or holes: [contracts-holes.md](contracts-holes.md) first, then
  [contracts.md](contracts.md) or [holes.md](holes.md).
- Commands and machine-readable output: [commands.md](commands.md) and
  [json-output.md](json-output.md), then the command-specific JSON page.
- Runtime and examples: [execution.md](execution.md) and [examples.md](examples.md).
- Rationale: [source-decisions.md](source-decisions.md).

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  older proposal history.
- Use command-specific JSON pages only after [json-output.md](json-output.md)
  routes the change.
- Use [overview.md](overview.md) only when you need the stability boundary or
  explicit non-goals.
- Use `../source-decisions/`, `../../proposals/`, `../../phases/`, or
  `../../reviews/` only after the current behavior page does not answer the
  question.
