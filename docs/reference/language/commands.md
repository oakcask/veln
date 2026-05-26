# Commands

This file routes command changes to the implemented CLI behavior without
requiring the full command reference on the first read.

## Read First

- `check`: source discovery, parse/semantic diagnostics, checked-core
  blockers, and check JSON output. Use [diagnostics-json.md](diagnostics-json.md)
  first for diagnostic shape, then
  [commands-full.md#veln-check](commands-full.md#veln-check) for exact command
  rules.
- `fmt`: whole-invocation parse gate, deterministic formatting, and comment
  preservation. Use [commands-full.md#veln-fmt](commands-full.md#veln-fmt)
  only when the route summary is not enough.
- `run`: entry resolution, argument conversion, static gates, JVM execution,
  and run JSON. Use [run-json.md](run-json.md) first for machine-readable
  output, then [commands-full.md#veln-run](commands-full.md#veln-run) for exact
  command rules.
- `test`: test and doctest selection, static gates, runtime failures, and test
  JSON. Use [test-json.md](test-json.md) first for machine-readable output,
  then [commands-full.md#veln-test](commands-full.md#veln-test) for exact
  command rules.
- `explain`: diagnostic catalog lookup. Use
  [commands-full.md#veln-explain](commands-full.md#veln-explain) when
  diagnostic catalog behavior is the task.

## Read When

- Use [json-output.md](json-output.md) to choose the implemented reference for
  `check --json`, `run --json`, or `test --json` output.
- Use [source-surface.md](source-surface.md) when command behavior depends on
  source syntax, doctest fences, or module declarations.

## Skip Unless Needed

- Use only the command section above that matches the task.
- Use [../source-decisions/commands-output.md](../source-decisions/commands-output.md)
  only when the implemented command reference does not explain why a boundary
  exists.
