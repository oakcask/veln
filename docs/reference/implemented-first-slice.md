# Implemented First-Slice Language Specification

Status: implemented
This is the stable entry point for behavior implemented in the current Veln
workspace. The full specification is split by category under
`language/`.

Use these files before changing code, tests, diagnostics, or samples. Use
[../proposals/grammar-target.md](../proposals/grammar-target.md) for the
broader first-slice language target, and use discussion results only for
rationale or future design work.

## Read First

- [language/README.md](language/README.md) routes the categorized language
  specification.
- [language/overview.md](language/overview.md) defines the implemented stability
  boundary.
- [language/source-surface.md](language/source-surface.md) defines implemented
  source syntax and grammar.
- [language/types.md](language/types.md) defines implemented annotations,
  inference, assignment, and operator typing.
- [language/diagnostics-json.md](language/diagnostics-json.md) defines
  `veln check --json` output.

## Read When

- Use [language/commands.md](language/commands.md) for CLI behavior and source
  discovery.
- Use [language/names-effects.md](language/names-effects.md) for name
  resolution and effect checking.
- Use [language/contracts-holes.md](language/contracts-holes.md) for contract
  validation, holes, and repair constraints.
- Use [language/test-json.md](language/test-json.md) for `veln test --json`
  output and bootstrap test discovery.
- Use [language/execution.md](language/execution.md) for checked-core, IR, and
  JVM backend boundaries.
- Use [language/source-decisions.md](language/source-decisions.md) for the
  dated decisions behind this implemented specification.

## Status Boundary

The split files describe implemented behavior only. Planned grammar,
discussion outcomes that are not implemented, runtime contract enforcement,
package manifests, imports, persistent build caches, and entry arguments are
outside this reference unless a categorized file states otherwise.
