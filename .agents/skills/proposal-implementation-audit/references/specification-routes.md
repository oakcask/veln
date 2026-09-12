# Proposal Specification Routes

Use the smallest route matching the implemented observable behavior:

- Shared command analysis, source discovery, checked-core readiness, typed-IR
  readiness, and command parity: start from
  `docs/reference/implemented-proposals/project-analysis-pipeline.md`, then use
  `docs/specification/commands.md`, `docs/specification/execution.md`, or
  `docs/specification/json-output.md` as applicable.
- Source syntax, tests, doctests, names, types, and effects: start from
  `docs/specification/topic-map.md#source-surface`.
- Runtime-failure doctest metadata and expected outcomes: use
  `docs/specification/source-surface.md`, `docs/specification/commands.md`, and
  `docs/specification/test-json.md` after implementation coverage exists.
- Repair candidates, satisfy constraints, and hole diagnostics: use
  `docs/specification/holes.md` and
  `docs/specification/diagnostics-json.md`.
- Contract predicates, static obligation classification, and result bindings:
  use `docs/specification/contracts.md`.
- Effect propagation or compiler-known calls: use
  `docs/specification/names-effects.md`.
- Command-specific machine-readable output: update
  `docs/specification/json-output.md` before command-specific JSON pages.
