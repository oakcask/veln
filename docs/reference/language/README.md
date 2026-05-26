# Language Specification

This directory routes implemented Veln language behavior. It records what the
current workspace supports, not every target tracked in `../../proposals/`.

## Read First

- [overview.md](overview.md): stability boundary and explicit non-goals.
- [topic-map.md](topic-map.md): choose the smallest topic page for a source,
  command, JSON, runtime, contract, or hole change.
- Open a short topic page before any matching `*-full.md` detail file.

## Fast Routes

- Source files, grammar, names, types, effects, contracts, or holes:
  [topic-map.md#source-surface](topic-map.md#source-surface) and
  [topic-map.md#contracts-and-holes](topic-map.md#contracts-and-holes).
- CLI behavior, diagnostics, run output, or test output:
  [topic-map.md#commands-and-output](topic-map.md#commands-and-output).
- Human diagnostic wording, related notes, spans, or stable diagnostic details:
  [diagnostics-json.md](diagnostics-json.md).
- Runtime behavior, examples, or rationale:
  [topic-map.md#runtime-examples-and-rationale](topic-map.md#runtime-examples-and-rationale).

## Read When

- Task-oriented topic selection: [topic-map.md](topic-map.md).
- Source syntax and grammar details: [source-surface.md](source-surface.md),
  [types.md](types.md), and [names-effects.md](names-effects.md).
- Contracts or holes: [contracts-holes.md](contracts-holes.md) first, then
  [contracts.md](contracts.md) or [holes.md](holes.md).
- Commands and machine-readable output: [commands.md](commands.md) and
  [json-output.md](json-output.md), then the command-specific JSON page.
- Human diagnostics: [diagnostics-json.md](diagnostics-json.md) for the
  structured behavior that must stay aligned with diagnostic output, then
  [source-decisions.md](source-decisions.md) only for rationale.
- Runtime and examples: [execution.md](execution.md) and [examples.md](examples.md).
- Rationale: [source-decisions.md](source-decisions.md).

## Update When

- A proposal becomes implemented behavior.
- A test, diagnostic, command, JSON schema, or example changes observable
  behavior.
- Planned rationale becomes implemented behavior or changes how users should
  read the language.
- After promoting proposal behavior, update the smallest topic page named by
  [topic-map.md](topic-map.md) and keep any remaining proposal work in
  [../../proposals/target-queue.md](../../proposals/target-queue.md).

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  older proposal history.
- Use command-specific JSON pages only after [json-output.md](json-output.md)
  routes the change.
- Use `../source-decisions/`, `../../proposals/`, `../../phases/`, or
  `../../reviews/` only after the current behavior page does not answer the
  question.
