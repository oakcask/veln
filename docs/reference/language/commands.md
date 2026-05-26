# Commands

This file routes command changes to the implemented CLI behavior without
requiring the full command reference on the first read.

## Read First

- `check`: source discovery, parse/semantic diagnostics, checked-core
  blockers, and check JSON output:
  [commands-full.md#veln-check---json-path](commands-full.md#veln-check---json-path).
- `fmt`: whole-invocation parse gate, deterministic formatting, and comment
  preservation:
  [commands-full.md#veln-fmt-path](commands-full.md#veln-fmt-path).
- `run`: entry resolution, argument conversion, static gates, JVM execution,
  and run JSON:
  [commands-full.md#veln-run---json-path----arg](commands-full.md#veln-run---json-path----arg).
- `test`: test and doctest selection, static gates, runtime failures, and test
  JSON:
  [commands-full.md#veln-test---json-target](commands-full.md#veln-test---json-target).
- `explain`: diagnostic catalog lookup:
  [commands-full.md#veln-explain---list-diagnostic-id](commands-full.md#veln-explain---list-diagnostic-id).

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
