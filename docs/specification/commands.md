---
review-when: The documented command behavior or executable command evidence changes.
---

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
  binary schema primitive spelling for supported compatibility fields and
  payloads, canonical bool `match` to `if` / `else` rewriting, with literal
  equality chains kept as direct literal `match` expressions, and canonical
  hash spelling for standalone and trailing line comments. Use
  [commands-full.md](commands-full.md) only when the route summary is not
  enough.
- `metrics`: advisory module dependency metrics, ABC size metrics, and
  experimental exact whole-body similarity for project-owned Veln source. It
  follows `check` source and project discovery for containing graph analysis
  and accepts `--json`. Human output prints dependency sections, ABC size, and
  then whole-body similarity with one primary declaration location and related
  declaration locations. `--write-baseline PATH` writes the current report as
  a reviewed baseline and refuses to overwrite an existing file. Without
  `--check`, it exits successfully when analysis completes even when
  dependency cycles, large ABC values, or duplicate whole bodies are present.
  With `--check`, `[tool.metrics] deny_cycles = "true"` makes dependency
  cycles an enforced project policy. `--baseline PATH` is valid only with
  `--check` and allows unchanged or reduced dependency cycles while rejecting
  cycle regressions. `[tool.metrics] max_findings = "N"` limits detailed
  human-output findings only; summaries, JSON arrays, policy evaluation, and
  baseline content still use the complete finding set. Similarity remains
  advisory during baseline checks. No enabled policy or invalid metrics policy
  configuration is a command error.
  Use [metrics-json.md](metrics-json.md) for machine-readable output.
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
  `EncodeError` value failures for schema-owned primitive representability,
  repeat count mismatches, length-bounded `ByteView` count mismatches,
  schema-owned dispatch unknown tags, dispatch length mismatches, and dispatch
  tag/payload mismatches, direct source-visible `EncodeError(...)` result failures,
  hand-written codec `EncodeStep::Invalid(EncodeError(...))` entry results,
  direct source-visible `DecodeError(...)` and
  `DecodeErrorWithReason(...)` result failures,
  source-visible `DecodeStep::Invalid(DecodeError(...))` and
  `DecodeStep::NeedMore(...)` entry results,
  HTTP/2 protocol-core failures
  including partial and invalid client connection prefaces, frame-size
  peer-limits with bounded frame-header byte previews, flow-control
  peer-limits with bounded DATA payload byte previews,
  header-list and header-table receive-limit peer-limits with bounded
  header-block byte previews,
  GOAWAY receive preserving already-admitted stream DATA and trailer HEADERS
  lifecycle while rejecting later peer-created streams above the recorded last
  stream id with bounded frame-header byte previews, repeated local outbound
  GOAWAY send-intents that preserve or narrow the locally recorded shutdown
  boundary, plus local outbound HEADERS, stream-level outbound
  `WINDOW_UPDATE`, and server-side outbound `PUSH_PROMISE` send-intents above
  received or locally sent GOAWAY boundaries, plus server-side outbound
  promised stream id ordering with retained connection state and focused
  human and JSON diagnostic projections, plus client-side outbound HEADERS
  local stream admission and retained stream-id ordering with focused human
  and JSON projections,
  standard helper-returned frame-size, SETTINGS value, and header-table
  runtime diagnostic payloads,
  HPACK dynamic index lookup failures with dynamic table entry counts, and
  HPACK dynamic table-size update placement and trailing-byte failures with
  frame and stream context,
  SETTINGS value range peer-limit, stream id domain
  failures with bounded frame-header byte previews, invalid connection-state
  and stream-state frame-kind failures with bounded frame-header byte
  previews, continuation-ordering and pending-byte close failures with
  bounded protocol-owned byte previews, unexpected SETTINGS ACK failures with
  bounded frame-header byte previews, and fixed payload-length failures
  including SETTINGS ACK, PING, GOAWAY, `RST_STREAM`, and `WINDOW_UPDATE`,
  plus invalid DATA padding and content-length body mismatches, with bounded
  payload byte-preview notes. `RST_STREAM` payload-length projection has
  checked human and JSON cases,
  and run JSON. Use
  [run-json.md](run-json.md) first for
  machine-readable output, then [commands-full.md](commands-full.md) for exact
  command rules. Standard helper calls for pending-byte close and partial
  preface failures return the same source-visible HTTP/2 protocol diagnostics.
  Human schema-owned byte diagnostics and HTTP/2 client connection preface,
  continuation-ordering, and pending-byte close protocol diagnostics render
  preview bytes as bounded lowercase hex pairs grouped with spaces and keep
  byte offsets, field paths, expected counts, actual counts, accepted ranges,
  actual values, matched prefix counts, byte values, active continuation
  state, and rule provenance in separate notes or structured details.
  Generated binary schema encode diagnostics, direct source-visible
  `EncodeError(...)` result failures, and
  `EncodeStep::Invalid(EncodeError(...))` entry diagnostics keep the primary
  message on the failed encode fact and put field path, reason or predicate
  details, and source-visible `EncodeError` value in related notes.
  Source-visible `RuntimeDiagnostic(..., RuntimeValueDiagnostic(...))`
  generated encode payloads keep the same public value diagnostic details
  while preserving the rendered `RuntimeDiagnostic(...)` value.
  Length-bounded `ByteView` encode count mismatches also put expected and
  actual byte counts, byte offset, and bounded nearby byte preview in related
  notes.
  Direct source-visible `DecodeError(...)`,
  `DecodeErrorWithReason(...)`, and
  `DecodeStep::Invalid(DecodeError(...))` entry diagnostics keep the primary
  message on the failed decode fact at the reported byte offset and put field
  path plus the source-visible `DecodeError` value in related notes. When the
  value is `DecodeErrorWithReason(...)`, the decode failure reason is also a
  related note and `details.byte_diagnostic.reason`. When that reason is a
  byte-helper failure message with registered helper context, related notes
  also include local byte offset, expected and available byte counts, and a
  bounded nearby-byte preview when available; `run --json` carries the same
  context as `details.byte_diagnostic.local_byte_offset`, `expected_count`,
  `available_count`, and `byte_preview`. The same projection applies when an
  ordinary decode function returns a codec-owned `Invalid(DecodeError(...))`
  or `Invalid(DecodeErrorWithReason(...))` result. Plain source-visible
  reasons preserve the codec-owned id and reason
  without helper-only related notes unless registered byte-helper context is
  present; direct `Result<_, DecodeError>` failures preserve codec-owned ids
  such as `codec.packet_kind_invalid` through the same focused human
  diagnostic shape. The checked packet-kind examples cover direct
  `DecodeErrorWithReason(...)` result failures and
  `Invalid(DecodeErrorWithReason(...))` entry results in
  `examples/specification/run/codec-packet-kind-invalid-direct-human/` and
  `examples/specification/run/codec-packet-kind-invalid-step-human/`.
  Codec-owned checksum mismatch failures with id
  `codec.checksum_mismatch` use `checksum mismatch at byte offset ...` as the
  primary human message and put field path, expected checksum, actual
  checksum, failure reason, and the source-visible `DecodeError` value in
  related notes; the checked direct result and `DecodeStep::Invalid(...)`
  examples are
  `examples/specification/run/codec-checksum-mismatch-direct-human/` and
  `examples/specification/run/codec-checksum-mismatch-step-human/`.
  Codec-owned length mismatch failures with id `codec.length_mismatch` use
  `length mismatch at byte offset ...` as the primary human message and put
  field path, expected length, actual length, failure reason, and the
  source-visible `DecodeError` value in related notes when the source-visible
  reason uses the narrow
  `expected_length=<n>; actual_length=<n>; reason=<text>` form; the checked
  direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-length-mismatch-direct-human/` and
  `examples/specification/run/codec-length-mismatch-step-human/`.
  Codec-owned payload length mismatch failures with id
  `codec.payload_length_mismatch` use
  `payload length mismatch at byte offset ...` as the primary human message
  and put field path, expected payload length, actual payload length, failure
  reason, and the source-visible `DecodeError` value in related notes when
  the source-visible reason uses the narrow
  `expected_payload_length=<n>; actual_payload_length=<n>; reason=<text>`
  form; the checked direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-payload-length-mismatch-direct-human/`
  and
  `examples/specification/run/codec-payload-length-mismatch-step-human/`.
  Codec-owned padding mismatch failures with id `codec.padding_mismatch` use
  `padding mismatch at byte offset ...` as the primary human message and put
  field path, expected padding length, actual padding length, failure reason,
  and the source-visible `DecodeError` value in related notes when the
  source-visible reason uses the narrow
  `expected_padding_length=<n>; actual_padding_length=<n>; reason=<text>`
  form; the checked direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-padding-mismatch-direct-human/` and
  `examples/specification/run/codec-padding-mismatch-step-human/`.
  Codec-owned integer range failures with id `codec.integer_out_of_range`
  use `integer out of range at byte offset ...` as the primary human message
  and put field path, byte width, expected integer range, actual decoded
  value, failure reason, and the source-visible `DecodeError` value in
  related notes when the source-visible reason uses the narrow form with
  `byte_width=<n>`, `min_value=<n>`, `max_value=<n>`, `actual_value=<n>`,
  and `reason=<text>`; the checked direct result and
  `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-integer-out-of-range-direct-human/` and
  `examples/specification/run/codec-integer-out-of-range-step-human/`.
  Codec-owned sequence mismatch failures with id `codec.sequence_mismatch`
  use `sequence mismatch at byte offset ...` as the primary human message and
  put field path, expected sequence, actual sequence, failure reason, and the
  source-visible `DecodeError` value in related notes when the source-visible
  reason uses the narrow
  `expected_sequence=<value>; actual_sequence=<value>; reason=<text>` form;
  the checked direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-sequence-mismatch-direct-human/` and
  `examples/specification/run/codec-sequence-mismatch-step-human/`.
  Codec-owned version mismatch failures with id `codec.version_mismatch`
  use `version mismatch at byte offset ...` as the primary human message and
  put field path, expected version, actual version, failure reason, and the
  source-visible `DecodeError` value in related notes when the source-visible
  reason uses the narrow
  `expected_version=<value>; actual_version=<value>; reason=<text>` form;
  the checked direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-version-mismatch-direct-human/` and
  `examples/specification/run/codec-version-mismatch-step-human/`.
  Codec-owned tag mismatch failures with id `codec.tag_mismatch` use
  `tag mismatch at byte offset ...` as the primary human message and put
  field path, expected tag, actual tag, failure reason, and the
  source-visible `DecodeError` value in related notes when the source-visible
  reason uses the narrow
  `expected_tag=<value>; actual_tag=<value>; reason=<text>` form; the checked
  direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-tag-mismatch-direct-human/` and
  `examples/specification/run/codec-tag-mismatch-step-human/`.
  Codec-owned magic mismatch failures with id `codec.magic_mismatch` use
  `magic mismatch at byte offset ...` as the primary human message and put
  field path, expected magic, actual magic, failure reason, and the
  source-visible `DecodeError` value in related notes when the source-visible
  reason uses the narrow
  `expected_magic=<value>; actual_magic=<value>; reason=<text>` form; the
  checked direct result and `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-magic-mismatch-direct-human/` and
  `examples/specification/run/codec-magic-mismatch-step-human/`.
  Codec-owned unsupported feature failures with id
  `codec.unsupported_feature` use
  `unsupported feature failed at byte offset ...` as the primary human
  message and put field path, unsupported feature, failure reason, and the
  source-visible `DecodeError` value in related notes when the
  source-visible reason uses the narrow
  `feature=<value>; reason=<text>` form; the checked direct result and
  `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-unsupported-feature-direct-human/` and
  `examples/specification/run/codec-unsupported-feature-step-human/`.
  Codec-owned trailing-input failures with id `codec.trailing_input` use
  `trailing input at byte offset ...` as the primary human message and put
  field path, consumed, available, and remaining byte counts, failure reason,
  and the source-visible `DecodeError` value in related notes when the
  source-visible reason uses the narrow
  `consumed_count=<n>; available_count=<n>; remaining_count=<n>; reason=<text>`
  form. Counts are projected only when remaining is positive and consumed plus
  remaining equals available. The checked direct result and
  `DecodeStep::Invalid(...)` examples are
  `examples/specification/run/codec-trailing-input-direct-human/` and
  `examples/specification/run/codec-trailing-input-step-human/`; the plain
  reason fallback is checked by
  `examples/specification/run/codec-trailing-input-plain-step-human/`. A
  source-visible `ByteView` range failure reports
  `codec.byte_range_out_of_bounds` at the requested byte offset and puts the
  requested count, available count, and bounded nearby byte preview in related
  notes. A source-visible
  `Err(RuntimeDiagnostic(id, message, RuntimeByteDiagnostic(...)))` value uses
  the same human byte-diagnostic rendering as value-carried runtime byte
  failures, with the id, byte offset, field path, counts, readiness,
  fixed-field expected and actual values, reason, and optional preview
  projected from the returned error value itself. Generated binary schema
  decode fixed-field mismatches return this payload directly and keep the
  focused `schema.fixed_field_mismatch` human diagnostic. Plain
  `Err(value)` values remain ordinary result failures. A source-visible
  `Err(RuntimeDiagnostic(id, message, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(...))))`
  value for unsupported-header-block, unsupported-static-index,
  malformed-string-length, malformed-raw-string, malformed-Huffman-padding,
  Huffman-EOS, and Huffman non-visible HPACK fixture ids, plus the
  source-visible HPACK static decoder `hpack.static.unsupported_index` id and
  malformed table-size update integers,
  uses the same focused HPACK fixture human diagnostic as the compatibility
  helper, with byte offset, observed header block size, observed first byte,
  expected fixture, codec module, and bounded byte preview projected from the
  returned error value. Checked focused examples cover both a direct returned
  diagnostic value and projection from the HTTP/2 protocol-core HPACK failure
  path.
  Source-visible HPACK static Huffman failures projected from the static
  boundary keep the same fields and use
  `codec_module = "hpack_static"`. The standalone source-visible HPACK static
  boundary case checks accepted static-name literal-with-indexing and
  literal-never-indexed inputs, accepted Huffman-marked literal values decoded
  through the HPACK static Huffman table for the three static-name forms, and
  malformed raw-length fallback for those forms. The aggregate HTTP/2
  protocol-core run case also checks source-visible HPACK static-name
  `:scheme` and `:authority` literal values in request header blocks through
  the existing request header-list validation path, including accepted raw
  `:scheme` values `http` and `https`, the checked Huffman-marked `https`
  value on completed HEADERS and final CONTINUATION paths, the checked
  Huffman-marked `:path: test` value on completed HEADERS and final
  CONTINUATION paths, accepted visible ASCII `:authority` values, and
  rejected visible ASCII values for both pseudo-headers. It also checks a
  source-visible HPACK static-name `content-length` literal in request header
  blocks across the
  literal-without-indexing, literal-with-indexing, and literal-never-indexed
  forms that do not require later fixture dynamic-table reuse; accepted
  visible ASCII decimal values update the existing content-length
  body-accounting state, while non-decimal visible values use the existing
  request header-list validation diagnostic. The aggregate case also checks
  ordinary `CONNECT` request-header validation on completed HEADERS and final
  CONTINUATION paths. A non-empty `:authority` without `:scheme` or `:path`
  is accepted; missing or empty `:authority` and present `:scheme` or `:path`
  use focused request-header diagnostics. The same aggregate run case also
  checks the source-visible HPACK Huffman encode boundary directly:
  successful calls print payload-only `ByteChunk` output for supported string
  and bounded byte input, while unsupported string input prints the returned
  HPACK fixture failure. The focused HPACK fixture boundary case checks the
  same payload-only boundary without routing through outbound header-list
  fixture encoding. The same HPACK fixture boundary case also checks a
  standalone source-visible static-indexed encode helper
  for exact HPACK static table fixed-value entries, including request
  pseudo-header, response pseudo-header, and ordinary-header examples, and
  keeps non-exact values for known static names on the fixture encode-failure
  path. It also checks a
  standalone source-visible `hpack_dynamic_core` dynamic
  indexed decode for multiple carried bounded entries, decode-count
  advancement after accepted reads, and the focused
  `hpack.fixture.dynamic_index_out_of_range` failure facts when an indexed
  byte asks past the carried table without advancing state. It also checks
  source-visible HPACK dynamic-table accounting helpers for entry-size
  calculation, newest-first insertion, retained older entries, table-size
  reduction eviction including a zero-size table, insertion-caused eviction,
  over-limit insertion, static-name literal-with-indexing insertion for
  `content-type: text`, later dynamic-indexed reuse through `0xbe`, accepted
  raw visible-ASCII literal-name fields across the literal-without-indexing,
  literal-with-indexing, and literal-never-indexed forms, including bounded
  Huffman-marked values accepted by the checked HPACK Huffman boundary,
  dynamic-table mutation only for literal-with-indexing, dynamic-indexed reuse
  of the inserted Huffman-valued raw literal, focused malformed-Huffman
  fallback projection, and final CONTINUATION routing through the
  source-visible raw literal-name boundary before fixture fallback. The same
  checked boundary accepts dynamic-name Huffman-marked values for
  literal-without-indexing, literal-with-indexing, and literal-never-indexed;
  only literal-with-indexing inserts the decoded value, and completed HEADERS
  plus final CONTINUATION routing use the source-visible boundary before
  fixture fallback.
  `RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicIndexDiagnostic(...))`,
  `RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicNameDiagnostic(...))`, and
  `RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureTableSizeUpdateDiagnostic(...))` additionally project the
  dynamic-index, dynamic-name continuation, and table-size update facts needed
  by those focused human diagnostics, including the trailing-byte table-size
  update diagnostic. The standard `hpack_fixture_*` reporting helpers return
  their HPACK fixture payloads directly as
  `Result<(), RuntimeDiagnostic>`, so their command-facing detail projection is
  derived from the returned value.
  Source-visible `Err(RuntimeDiagnostic(...))` HTTP/2 protocol
  payload projections for pending-byte close, partial and invalid client
  connection preface failures, continuation ordering, invalid frame kind,
  frame-size exceeded, header-list receive-limit failures, SETTINGS value
  range peer-limit failures, stream id domain failures, fixed payload length,
  DATA padding,
  flow-control window, content-length mismatch, request and response
  header-list validation, invalid
  `WINDOW_UPDATE` increment, unexpected SETTINGS ACK, invalid PRIORITY dependency,
  stream-after-GOAWAY,
  header-table receive-limit failures, and concurrent-stream receive-limit failures
  likewise use the same human runtime diagnostic rendering as the
  compatibility helpers, with the stable id, protocol facts, provenance,
  decoded header names, and bounded byte preview projected from the returned
  value. The request and response header-list validation projections carry a
  bounded inspected header-block preview. The frame-size and
  concurrent-stream receive-limit
  projections include bounded byte previews for inspected frame headers from
  the returned `RuntimeDiagnostic(...)` value, and stream-after-GOAWAY
  projections include the bounded inspected frame-header preview or empty local
  outbound preview and active shutdown label carried by the returned value.
  The standard
  `http2::diagnostic::protocol_invalid_preface(...)`,
  `http2::diagnostic::protocol_initial_peer_settings_required(...)`,
  `http2::diagnostic::protocol_continuation_expected(...)`, and
  `http2::diagnostic::protocol_invalid_frame_kind(...)`,
  `http2::diagnostic::protocol_invalid_stream_id(...)`,
  `http2::diagnostic::peer_limit_frame_size_exceeded(...)`,
  `http2::diagnostic::peer_limit_header_list_size_exceeded(...)`,
  `http2::diagnostic::peer_limit_header_table_size_exceeded(...)`,
  `http2::diagnostic::peer_limit_concurrent_streams_exceeded(...)`, and
  `http2::diagnostic::peer_limit_settings_value_out_of_range(...)`,
  `http2::diagnostic::protocol_invalid_window_update_increment(...)`,
  `http2::diagnostic::protocol_invalid_data_padding(...)`,
  `http2::diagnostic::protocol_content_length_mismatch(...)`,
  `http2::diagnostic::protocol_unexpected_settings_ack(...)`,
  `http2::diagnostic::protocol_settings_not_allowed_for_endpoint(...)`,
  `http2::diagnostic::protocol_invalid_priority_dependency(...)`,
  `http2::diagnostic::protocol_stream_after_goaway(...)`,
  `http2::diagnostic::peer_limit_flow_control_window_exceeded(...)`,
  `http2::diagnostic::protocol_invalid_request_header_list(...)`, and
  `http2::diagnostic::protocol_invalid_response_header_list(...)` helpers also return
  this payload form, so their human runtime diagnostics are rendered from the
  returned value rather than from helper-local registration.
  Checked byte write
  conversion failures report
  `codec.byte_write_value_unrepresentable` and put the helper name, supplied
  value, accepted range, width, byte order, and source-visible `Err` value in
  related notes.
  A hand-written codec boundary that projects an oversized decoded consumed
  count as `codec.consumed_count_invalid` uses this shape and is not reported
  as retryable readiness. Human output keeps the primary message focused on
  the invalid consumed-count fact, puts field path, supplied view length,
  actual consumed count, reason, and source-visible `DecodeErrorWithReason`
  value in related notes, and is checked by
  `examples/specification/run/codec-consumed-count-invalid-human/`.
  `DecodeStep::NeedMore(...)` entry diagnostics report
  `codec.incomplete_input` at the closed-input byte boundary and put
  readiness, requested count when present, and the source-visible
  `DecodeStep` value in related notes.
  Transport runtime
  failures from descriptor-backed
  receive/send calls, fixture-backed or production loopback socket
  listen/accept/read/write and address metadata calls, and relative timeout
  or deadline calls stay runtime errors.
- `test`: test and doctest selection, static gates, bounded `-j` / `--jobs`
  case execution, the serial `--jobs 1` compatibility route, deterministic
  ordered reporting, direct JVM classfile execution without an ordinary Java
  source compiler requirement, `runtime=contract`, `runtime=ensure`, and
  `runtime=result` doctest expectations, runtime failures, captured stdio
  events, and test JSON. Use
  [source-surface.md](source-surface.md) first for doctest fence metadata,
  [test-json.md](test-json.md) first for
  machine-readable output, then [commands-full.md](commands-full.md) for exact
  command rules. The checked examples
  `../../examples/specification/test/parallel-jobs-one-json/`,
  `../../examples/specification/test/parallel-jobs-two-json/`, and
  `../../examples/specification/test/parallel-jobs-auto-json/` show that
  serial, bounded, and automatic job modes preserve ordered case records.
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
  identities. The toolchain-owned `std` package cannot be declared as a
  dependency and is never written to `veln.lock`. Use
  [commands-full.md#veln-package-lock](commands-full.md#veln-package-lock)
  when changing package-manager command behavior.
- `lsp`: stdio language-server startup for editor semantic highlighting and
  diagnostics. Use [editor-support.md](editor-support.md) first for editor
  protocol behavior.

## Read When

The HTTP/2 run examples cover extended CONNECT negotiation through
`SETTINGS_ENABLE_CONNECT_PROTOCOL`, including accepted HEADERS and final
CONTINUATION completion, local server advertisement, endpoint-role and value
rejection, required `:protocol`, `:scheme`, `:path`, and `:authority` facts,
and the unchanged ordinary CONNECT path. Focused human and JSON cases expose
the failed header fact, header name, negotiation state, and rule provenance.

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
