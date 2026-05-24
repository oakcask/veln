# Reference

Stable reference material lives here. Read these files when you need behavior
implemented in the current workspace without the historical discussion that led
to it. Use these files before changing code, tests, diagnostics, or samples.

## Read First

- [language/README.md](language/README.md) routes the categorized language
  specification for the implemented first slice.
- [language/overview.md](language/overview.md) defines the implemented
  stability boundary.
- [language/source-surface.md](language/source-surface.md) defines implemented
  source syntax and grammar.
- [language/types.md](language/types.md) defines implemented annotations,
  inference, assignment, and operator typing.
- [language/diagnostics-json.md](language/diagnostics-json.md) defines
  `veln check --json` output.
- [source-decisions/README.md](source-decisions/README.md) lists implemented
  decision records for rationale and compatibility context.
- [bibliography/source-families.md](bibliography/source-families.md) groups the
  research references that support design decisions.

## Read When

- Use `language/` before changing code, tests, diagnostics, or samples.
- Use `../proposals/grammar-target.md` when you need the broader first-slice
  language target, including planned syntax not yet implemented.
- Use `source-decisions/` for implemented decision rationale.
- Use `../proposals/agent-language-spec-wall/` for planned or incomplete
  decision rationale.
- Use `../phases/` for implementation plans and completion notes.
- Use `../reviews/` for current implementation gaps and verification findings.

## Status Boundary

The split language files describe implemented behavior only. Planned grammar,
discussion outcomes that are not implemented, runtime contract enforcement,
package manifests, imports, persistent build caches, and non-string entry
argument conversion are outside this reference unless a categorized file states
otherwise.
