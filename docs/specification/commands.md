# Commands

This file routes command changes to the implemented CLI behavior without
requiring the full command reference on the first read.

## Read First

- `check`: source discovery, parse/semantic diagnostics, checked-core
  blockers, and check JSON output. Use [diagnostics-json.md](diagnostics-json.md)
  first for diagnostic shape, then
  [commands-full.md](commands-full.md) for exact command rules.
- `fmt`: whole-invocation parse gate, deterministic formatting, tab-based
  canonical indentation, `match` arm indentation, and comment preservation.
  Use [commands-full.md](commands-full.md) only when the route summary is not
  enough.
- `run`: entry resolution, argument conversion, static gates, direct JVM
  classfile execution without an ordinary Java source compiler requirement, and
  run JSON. Use [run-json.md](run-json.md) first for machine-readable output,
  then [commands-full.md](commands-full.md) for exact command rules.
- `test`: test and doctest selection, static gates, direct JVM classfile
  execution without an ordinary Java source compiler requirement, runtime
  failures, and test JSON. Use [test-json.md](test-json.md) first for
  machine-readable output, then [commands-full.md](commands-full.md) for exact
  command rules.
- `repair`: preview or apply one safe advisory hole repair candidate from
  current source analysis or saved repair JSON input, selected by `repair_id`
  or `source_candidate_id` when `--candidate` is present. Use
  [repair-candidates.md](repair-candidates.md) for the candidate and input
  boundary, and [repair-json.md](repair-json.md) for machine-readable output.
- `explain`: diagnostic catalog lookup. Use
  [commands-full.md](commands-full.md) when diagnostic catalog behavior is the
  task.
- `lsp`: stdio language-server startup for editor semantic highlighting. Use
  [editor-support.md](editor-support.md) first for semantic-token behavior.

## Read When

- Use [json-output.md](json-output.md) to choose the implemented reference for
  `check --json`, `run --json`, `test --json`, or `repair --json` output.
- Use [source-surface.md](source-surface.md) when command behavior depends on
  source syntax, doctest fences, or module declarations.

## Skip Unless Needed

- Use only the command section above that matches the task.
- Use [../reference/source-decisions/commands-output.md](../reference/source-decisions/commands-output.md)
  only when the implemented command reference does not explain why a boundary
  exists.
