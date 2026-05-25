# Reference

Stable reference material lives here. Read these files when you need behavior
implemented in the current workspace without the historical discussion that led
to it. Use these files before changing code, tests, diagnostics, or samples.

## Read First

- [language/README.md](language/README.md): current language behavior.
- [language/topic-map.md](language/topic-map.md): task-oriented route to the
  smallest language reference page.
- [language/source-decisions.md](language/source-decisions.md): short route to
  rationale when current behavior needs context.

## Read When

- Source language, contract, hole, command, JSON-output, runtime, or example
  changes: use [language/topic-map.md](language/topic-map.md), then open the
  selected short reference page.
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
