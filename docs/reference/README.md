# Reference

Stable reference material lives here. Read these files when you need behavior
implemented in the current workspace without the historical discussion that led
to it. Use these files before changing code, tests, diagnostics, or samples.

## Read First

- [language/README.md](language/README.md): current language behavior.
- [language/source-decisions.md](language/source-decisions.md): short route to
  rationale when current behavior needs context.

## Read When

- Source language changes: [language/source-surface.md](language/source-surface.md),
  [language/types.md](language/types.md), and
  [language/names-effects.md](language/names-effects.md).
- Contract or hole changes: [language/contracts-holes.md](language/contracts-holes.md)
  routes the short and full references.
- Command and JSON-output changes: [language/commands.md](language/commands.md),
  [language/diagnostics-json.md](language/diagnostics-json.md),
  [language/run-json.md](language/run-json.md), and
  [language/test-json.md](language/test-json.md).
- Execution or examples: [language/execution.md](language/execution.md) and
  [language/examples.md](language/examples.md).
- Rationale: [language/source-decisions.md](language/source-decisions.md) first,
  then [source-decisions/README.md](source-decisions/README.md) if detail is
  needed.
- Research-source routes behind source decisions:
  [bibliography/README.md](bibliography/README.md).
- Planning, review, or proposal work: use `../proposals/`, `../phases/`, or
  `../reviews/` after checking the current reference.

## Status Boundary

The split language files describe implemented behavior only. Planned grammar,
discussion outcomes that are not implemented, package manifests beyond
implemented module validation, imports beyond current aliases, and persistent
build caches are outside this reference unless a categorized file states
otherwise. Use [../document-status.md](../document-status.md) when moving text
between proposal, review, phase, and reference areas.
