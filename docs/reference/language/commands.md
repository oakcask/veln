# Commands

This file routes command changes to the implemented CLI behavior without
requiring the full command reference on the first read.

## Read First

- `check`: source discovery, parse/semantic diagnostics, checked-core
  blockers, and check JSON output.
- `fmt`: whole-invocation parse gate, deterministic formatting, and comment
  preservation.
- `run`: entry resolution, argument conversion, static gates, JVM execution,
  and run JSON.
- `test`: test and doctest selection, static gates, runtime failures, and test
  JSON.
- `explain`: diagnostic catalog lookup.

Open [commands-full.md](commands-full.md) for the command-specific rules.

## Read When

- Use [diagnostics-json.md](diagnostics-json.md) for `check --json` envelope
  and diagnostic field stability.
- Use [run-json.md](run-json.md) for `run --json` output records.
- Use [test-json.md](test-json.md) for `test --json` selection, case, summary,
  failure, and error records.
- Use [source-surface.md](source-surface.md) when command behavior depends on
  source syntax, doctest fences, or module declarations.

## Skip Unless Needed

- Use [commands-full.md](commands-full.md) only when changing command behavior
  or checking a command-specific gate.
- Use [../source-decisions/commands-output.md](../source-decisions/commands-output.md)
  only when the implemented command reference does not explain why a boundary
  exists.
