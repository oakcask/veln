---
role: routing
update-when: A specification route is added, moved, reclassified, or no longer points to current behavior.
---

# Language Specification

This directory routes the current implemented Veln language specification. It
keeps only the latest behavior supported by the workspace, not every target
tracked in `../proposals/`.

## Read First

- [overview.md](overview.md): stability boundary and explicit non-goals.
- [topic-map.md](topic-map.md): choose the smallest topic page for a source,
  command, JSON, runtime, contract, or hole change.
- Keep prose thin: prefer executable or mechanically checked specifications for
  behavior detail, and use prose pages for routing, summary, and explanation.
- Specify externally observable behavior declaratively. Explain internal
  algorithms or ordered procedures only when needed, using Simplified
  Technical English style.
- Prefer executable or checked specification evidence for behavior changes:
  `../../examples/specification/`, generated grammar from
  `source-surface-executable.pl`, compiler tests, or CLI harness cases.
- Open a short topic page before any matching `*-full.md` detail file.

## Fast Routes

- Source files, grammar, names, types, effects, contracts, or holes:
  [topic-map.md#source-surface](topic-map.md#source-surface) and
  [topic-map.md#contracts-and-holes](topic-map.md#contracts-and-holes).
- Editor highlighting and semantic token behavior:
  [editor-support.md](editor-support.md).
- Package snapshots, package documentation catalogs, and canonical
  virtual-source resolution: [package-snapshots.md](package-snapshots.md),
  [package-documentation.md](package-documentation.md), then
  [package-virtual-sources.md](package-virtual-sources.md).
- CLI behavior, diagnostics, run output, or test output:
  [topic-map.md#commands-and-output](topic-map.md#commands-and-output).
- MCP workspace selection, tool schemas, refresh behavior, saved diagnostics,
  saved definitions, and saved function references:
  [mcp.md](mcp.md).
- Human diagnostic wording, related notes, spans, or stable diagnostic details:
  [diagnostics-json.md](diagnostics-json.md).
- Advisory hole repair candidates in `check --json`:
  [repair-candidates.md](repair-candidates.md).
- Applying-command gates for `repair --apply`:
  [repair-application.md](repair-application.md).
- Runtime behavior, examples, or rationale:
  [topic-map.md#runtime-examples-and-rationale](topic-map.md#runtime-examples-and-rationale).

## Read When

- Unknown implemented-behavior topic: [topic-map.md](topic-map.md).
- Source syntax and grammar details: [source-surface.md](source-surface.md),
  [types.md](types.md), and [names-effects.md](names-effects.md).
- Contracts or holes: [contracts-holes.md](contracts-holes.md) first, then
  [contracts.md](contracts.md) or [holes.md](holes.md).
- Commands and machine-readable output: [commands.md](commands.md) and
  [json-output.md](json-output.md), then the command-specific JSON page.
- Human diagnostics: [diagnostics-json.md](diagnostics-json.md) for the
  structured behavior that must stay aligned with diagnostic output, then
  [source-decisions.md](source-decisions.md) only for rationale.
- Repair candidates and application gates:
  [repair-candidates.md](repair-candidates.md) for advisory candidates, then
  [repair-application.md](repair-application.md) only for applying-command
  gates.
- Runtime and examples: [execution.md](execution.md) and [examples.md](examples.md).
- Editor support: [editor-support.md](editor-support.md).
- MCP workspace project inventory, saved diagnostics, saved definitions, and
  saved function references:
  [mcp.md](mcp.md).
- Rationale: [source-decisions.md](source-decisions.md).

## Update When

- A proposal becomes implemented behavior.
- A test, diagnostic, command, JSON schema, or example changes observable
  behavior.
- Executable or mechanically checked evidence changes in a way that affects the
  behavior described by a prose specification page.
- A prose page repeats behavior that is now covered by executable evidence; in
  that case, reduce the prose to a route, summary, or note.
- Planned rationale becomes implemented behavior or changes how users should
  read the language.
- After promoting proposal behavior, update the smallest topic page named by
  [topic-map.md](topic-map.md) and keep any remaining proposal work under
  `../proposals/`.

## Skip Unless Needed

- Use `source-surface.md` for the implemented source grammar before checking
  proposal directories.
- Do not rely on prose alone for behavior that can be captured in
  `../../examples/specification/`, `source-surface-executable.pl`, compiler
  tests, or CLI harness cases.
- Use command-specific JSON pages only after [json-output.md](json-output.md)
  routes the change.
- Use `../reference/source-decisions/` or `../proposals/` only after the
  current behavior page does not answer the question.
- Do not open full detail files until the matching short topic page points to a
  section that matters.
