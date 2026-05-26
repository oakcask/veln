# Source Decision Topic Map

Use this map only after the current language reference names a boundary but
does not explain why it exists. Start with a category page, then open one
record only when the category page points to it.

## Read First

- Current behavior: [../language/topic-map.md](../language/topic-map.md).
- Language-facing rationale route:
  [../language/source-decisions.md](../language/source-decisions.md).
- Category index: [README.md](README.md).

## Route Rule

Open one category page from the sections below, then one `result-*.md` record
only if that category page names it. Return to `../language/` before changing
implemented behavior.

## Source Language

- Syntax, blocks, compact functions, modules, methods, pipelines, tests:
  [language-surface.md#source-shape](language-surface.md#source-shape).
- Types, values, public function boundaries, ADTs:
  [language-surface.md#types-and-values](language-surface.md#types-and-values).
- Contracts, holes, postcondition result bindings:
  [language-surface.md#contracts-and-holes](language-surface.md#contracts-and-holes).
- Names, effects, prelude helpers, scoping:
  [language-surface.md#names-and-effects](language-surface.md#names-and-effects).

## Commands And Output

- CLI command shape and discovery:
  [commands-output.md#commands-and-discovery](commands-output.md#commands-and-discovery).
- Diagnostics and command JSON:
  [commands-output.md#json-output](commands-output.md#json-output).
- Tests and doctests:
  [commands-output.md#tests-and-doctests](commands-output.md#tests-and-doctests).
- Runtime output:
  [commands-output.md#runtime-output](commands-output.md#runtime-output).

## Implementation Boundaries

- Architecture, AST, metadata, and runtime targets:
  [implementation-boundaries.md#architecture-and-ast](implementation-boundaries.md#architecture-and-ast).
- Runtime, compatibility, transitive effects, and value freezing:
  [implementation-boundaries.md#runtime-boundaries](implementation-boundaries.md#runtime-boundaries).
- Documentation placement, comparison, repair, and severity policy:
  [process-rationale.md](process-rationale.md).

## Skip Unless Needed

- Do not scan `result-*.md` files directly.
- Do not use source decisions as implementation status when `../language/`
  says otherwise.
