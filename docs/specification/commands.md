# Commands

This file routes command changes to the implemented CLI behavior without
requiring the full command reference on the first read.

## Read First

- `check`, `run`, `test`, and `repair` share the project analysis path for
  source discovery, parse-clean surface loading, semantic diagnostics,
  checked-core readiness, and selected-entry typed-IR readiness. Command
  sections below cover only their selection, output, execution, or write
  policy. Use
  [commands-full.md#shared-command-analysis](commands-full.md#shared-command-analysis)
  only when changing the shared path itself.
- Command help: top-level help, subcommand help, and help-topic errors are
  implemented command behavior. Use
  [commands-full.md#command-help](commands-full.md#command-help) when changing
  help parsing or output.
- `check`: source discovery, source path derived local module identity,
  manifest dependency metadata validation, path dependency source loading for
  external imports, parse/semantic diagnostics, checked-core blockers, and
  check JSON output.
  Use [diagnostics-json.md](diagnostics-json.md) first for diagnostic shape,
  then
  [commands-full.md](commands-full.md) for exact command rules.
- `fmt`: whole-invocation parse gate, deterministic formatting, tab-based
  canonical indentation, schema layout, `match` arm indentation, and canonical
  hash spelling for standalone and trailing line comments. Use
  [commands-full.md](commands-full.md) only when the route summary is not
  enough.
- `doc`: generated Markdown documentation from selected source files,
  package/tool manifest metadata, documentation comments, public API
  declarations including public schemas and schema aliases, schema references,
  contracts, doctest fences, and ADR-lite records. Use
  [commands-full.md](commands-full.md) when changing generated documentation
  output.
- `run`: entry resolution, fixed and variadic entry argument conversion,
  static gates, direct JVM classfile execution without an ordinary Java source
  compiler requirement,
  human runtime diagnostics for closed-input `ByteView` read truncation,
  schema fixed-field mismatch, binary schema field truncation, reserved-bit
  mismatch, integer range failure, field-local validation failure,
  closed-dispatch unknown tag
  failures, payload length boundary failures, schema length/count
  division-by-zero failures, generated binary schema
  `EncodeError` value failures for primitive representability, dispatch
  unknown tags, dispatch length mismatches, and dispatch tag/payload
  mismatches, hand-written codec `EncodeStep::Invalid(EncodeError(...))`
  entry results, source-visible
  `DecodeStep::Invalid(DecodeError(...))` and
  `DecodeStep::NeedMore(...)` entry results,
  HTTP/2 protocol-core failures
  including partial and invalid client connection prefaces, frame-size and
  flow-control peer-limits, header-list and header-table receive-limit
  peer-limits with bounded header-block byte previews,
  HPACK dynamic table-size update placement failures with frame and stream
  context,
  SETTINGS value range peer-limit, stream id domain
  failures with bounded frame-header byte previews, invalid connection-state
  and stream-state frame-kind failures with bounded frame-header byte
  previews, unexpected SETTINGS ACK failures with bounded frame-header byte
  previews, and fixed payload-length failures
  including SETTINGS ACK, PING, GOAWAY, `RST_STREAM`, and `WINDOW_UPDATE`,
  plus invalid DATA padding, with bounded payload byte-preview notes,
  and run JSON. Use
  [run-json.md](run-json.md) first for
  machine-readable output, then [commands-full.md](commands-full.md) for exact
  command rules. Human schema-owned byte diagnostics and HTTP/2 client
  connection preface protocol diagnostics render preview bytes as bounded
  lowercase hex pairs grouped with spaces and keep byte offsets, field paths,
  expected counts, actual counts, accepted ranges, actual values, matched
  prefix counts, byte values, and rule provenance in separate notes or
  structured details. Generated binary schema encode diagnostics and
  `EncodeStep::Invalid(EncodeError(...))` entry diagnostics keep the primary
  message on the failed encode fact and put field path, reason or predicate
  details, and source-visible `EncodeError` value in related notes.
  Length-bounded `ByteView` encode count mismatches also put expected and
  actual byte counts, byte offset, and bounded nearby byte preview in related
  notes.
  `DecodeStep::Invalid(DecodeError(...))` entry diagnostics keep the primary
  message on the failed decode fact at the reported byte offset and put field
  path plus the source-visible `DecodeError` value in related notes. A
  source-visible `ByteView` range failure reports
  `codec.byte_range_out_of_bounds` at the requested byte offset and puts the
  requested count, available count, and bounded nearby byte preview in related
  notes. Checked byte write conversion failures report
  `codec.byte_write_value_unrepresentable` and put the helper name, supplied
  value, accepted range, width, byte order, and source-visible `Err` value in
  related notes.
  A hand-written codec boundary that projects an oversized decoded consumed
  count as `codec.consumed_count_invalid` uses this shape and is not reported
  as retryable readiness.
  `DecodeStep::NeedMore(...)` entry diagnostics report
  `codec.incomplete_input` at the closed-input byte boundary and put
  readiness, requested count when present, and the source-visible
  `DecodeStep` value in related notes.
  Transport runtime
  failures from descriptor-backed
  receive/send calls, fixture-backed socket listen/accept/read/write calls,
  and relative timeout or deadline calls stay runtime errors.
- `test`: test and doctest selection, static gates, direct JVM classfile
  execution without an ordinary Java source compiler requirement,
  `runtime=contract`, `runtime=ensure`, and `runtime=result` doctest
  expectations, runtime failures, and test JSON. Use
  [source-surface.md](source-surface.md) first for doctest fence metadata,
  [test-json.md](test-json.md) first for
  machine-readable output, then [commands-full.md](commands-full.md) for exact
  command rules.
- `repair`: preview, apply one safe advisory hole repair candidate, or apply
  one explicitly confirmed manual-review candidate with override recording. Use
  [repair-candidates.md](repair-candidates.md) for candidate input and
  selection concepts, [repair-application.md](repair-application.md) for write
  gates, and [repair-json.md](repair-json.md) for machine-readable output.
- `explain`: diagnostic catalog lookup. Use
  [commands-full.md](commands-full.md) when diagnostic catalog behavior is the
  task.
- `package lock`: path, git, vendor, and mirror dependency graph lockfile
  writes, including incompatible source rejection for repeated package
  identities. Use
  [commands-full.md#veln-package-lock](commands-full.md#veln-package-lock)
  when changing package-manager command behavior.
- `lsp`: stdio language-server startup for editor semantic highlighting and
  diagnostics. Use [editor-support.md](editor-support.md) first for editor
  protocol behavior.

## Read When

- Use [json-output.md](json-output.md) to choose the implemented reference for
  `check --json`, `run --json`, `test --json`, or `repair --json` output.
- Use [source-surface.md](source-surface.md) when command behavior depends on
  source syntax, doctest fences, or path-derived module identity.
- Use
  [../reference/implemented-proposals/formatter-stabilization.md](../reference/implemented-proposals/formatter-stabilization.md)
  only when auditing the implemented formatter stabilization proposal record.

## Skip Unless Needed

- Use only the command section above that matches the task.
- Use [../reference/source-decisions/commands-output.md](../reference/source-decisions/commands-output.md)
  only when the implemented command reference does not explain why a boundary
  exists.
