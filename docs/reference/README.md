# Reference

Stable reference material lives here. Read these files when you need behavior
implemented in the current workspace without the historical discussion that led
to it. Use these files before changing code, tests, diagnostics, or samples.

## Read First

- [language/README.md](language/README.md) routes the categorized language
  specification for the implemented first slice.
- [source-decisions/README.md](source-decisions/README.md) lists implemented
  decision records for rationale and compatibility context.
- [bibliography/README.md](bibliography/README.md) routes research references
  that support design decisions.

## Common Routes

- Source syntax: [language/source-surface.md](language/source-surface.md)
- Type behavior: [language/types.md](language/types.md)
- Contracts and holes: [language/contracts-holes.md](language/contracts-holes.md)
- Contracts only: [language/contracts.md](language/contracts.md)
- Holes only: [language/holes.md](language/holes.md)
- Check output: [language/diagnostics-json.md](language/diagnostics-json.md)
- Run output: [language/run-json.md](language/run-json.md)
- Test output: [language/test-json.md](language/test-json.md)
- Rationale by topic:
  [source-decisions/README.md](source-decisions/README.md)

## Read When

- Use `language/` before changing code, tests, diagnostics, or samples.
- Use `language/source-surface.md` for implemented first-slice source grammar.
- Use `language/diagnostics-json.md` for `veln check --json` output.
- Use `source-decisions/` for implemented decision rationale.
- Use `../proposals/agent-language-spec-wall/` for planned or incomplete
  decision rationale.
- Use `../phases/` for implementation plans and completion notes.
- Use `../reviews/` for current implementation gaps and verification findings.

## Status Boundary

The split language files describe implemented behavior only. Planned grammar,
discussion outcomes that are not implemented, package manifests beyond
implemented module validation, imports beyond current aliases, and persistent
build caches are outside this reference unless a categorized file states
otherwise.
