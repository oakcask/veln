# Specification Examples

These examples are executable language fixtures. Each case is ordinary Veln
source plus a small `case.toml` manifest that the CLI toolchain harness runs
against the built `veln` binary.

## Read First

- Use this directory when behavior should be visible as source code and checked
  through the public CLI.
- Treat these cases as executable specification evidence for user-visible
  language and CLI behavior. Keep matching prose in `../../docs/specification/`
  aligned with the observable expectations here.
- Prefer adding or improving a case here over expanding prose when behavior is
  observable through source, diagnostics, command output, JSON, formatter
  output, generated docs, runtime output, LSP JSON-RPC, tests, or repair output.
- Keep case names grouped by command surface: `check`, `doc`, `fmt`,
  `explain`, `lsp`, `mcp`, `metrics`, `run`, `test`, `repair`, and
  `package`.
- Put expected observable behavior in `case.toml`; keep `.veln` files readable
  as examples of the language feature.

## Case Kinds

- `check/`: static diagnostics and successful static validation.
- `doc/`: generated Markdown documentation from source and manifest metadata.
- `fmt/`: deterministic source formatting and whole-invocation write gates.
- `explain/`: diagnostic catalog lookup and command-line errors.
- `lsp/`: editor JSON-RPC behavior exposed by the CLI.
- `mcp/`: agent-facing MCP JSON-RPC behavior exposed by the CLI.
- `metrics/`: advisory source dependency metrics and dependency-cycle policy
  checks exposed by the CLI.
- `run/`: executable entry points, runtime behavior, and runtime failures.
- `test/`: discovered tests, doctests, captured stdio, and test JSON behavior.
- `repair/`: advisory repair preview and repair JSON behavior.
- `package/`: package-manager command workflows and lockfile writes.

## Binary Schema Notes

- Public schema examples should apply schemas through explicit
  `decode Schema from view at base_offset` and `encode Schema from value`
  expressions, or through ordinary Veln functions that wrap those expressions.
- Existing cases that still call generated schema helper names are
  compatibility-only or diagnostic fixtures for the migration boundary. Keep
  them readable as legacy acceptance evidence, but do not use them as the
  teaching surface for normal schema application.
- Schema operations use the schema-local visible record shape.
- Use ordinary Veln functions to project between schema-local records and
  domain records at schema-operation boundaries.
- `check/schema-map-to-rejected/`,
  `check/schema-map-to-selector-rejected/`, and
  `check/schema-map-to-inverse-rejected/` pin parser rejection for plain,
  selected, and inverse schema-level `map to` forms.
- `run/schema-decode-expression/`, `run/schema-encode-expression/`, and
  `run/binary-schema-local-projection-boundary/` pin explicit schema
  operations combined with ordinary projection functions.

## Placement Guidelines

- Add cases here only when the fixture demonstrates a language or public CLI
  behavior that is useful to read as Veln source.
- A runtime case may require the current execution toolchain, but its expected
  behavior must stay phrased as source-level output, diagnostics, exit status,
  or command JSON.
- Do not add cases whose main purpose is to verify backend-private mechanics:
  artifact layout, classfile emission or validation, generated helper names,
  cache reuse, backend-specific limits, host tool setup, or other implementation
  details, except for narrowly named compatibility or diagnostic migration
  fixtures.
- Put backend invariants in backend crate tests, and put low-level CLI edge
  cases in CLI toolchain cases.
