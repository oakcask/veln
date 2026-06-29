# Runtime Diagnostic HPACK Helper Payload

Status: implemented

This record preserves the completed HPACK fixture standard helper slice from
the runtime diagnostic payload proposal. Current behavior is specified by
`../../specification/run-json.md`, `../../specification/commands.md`,
`../../specification/execution.md`, and checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The standard HPACK fixture reporting helpers now return
`Result<(), RuntimeDiagnostic>` directly:

- `hpack_fixture_unsupported_header_block(...)`
- `hpack_fixture_malformed_string_length(...)`
- `hpack_fixture_malformed_raw_string_value(...)`
- `hpack_fixture_malformed_huffman_padding(...)`
- `hpack_fixture_huffman_eos_symbol(...)`
- `hpack_fixture_huffman_non_visible_value(...)`
- `hpack_fixture_dynamic_index_out_of_range(...)`
- `hpack_fixture_table_size_update_malformed(...)`
- `hpack_fixture_table_size_update_not_at_start(...)`

Each helper returns an `Err(RuntimeDiagnostic(...))` value with the matching
`RuntimeHpackFixtureDiagnostic(...)`,
`RuntimeHpackFixtureDynamicIndexDiagnostic(...)`, or
`RuntimeHpackFixtureTableSizeUpdateDiagnostic(...)` payload. Command
projection keeps the rendered payload in `details.value` and derives
`details.protocol_diagnostic` from the returned value. The JVM runtime no
longer carries a message-keyed protocol diagnostic side table for these
standard helpers.

Plain non-diagnostic `Err(value)` values remain ordinary result failures and
do not opt into HPACK fixture diagnostic projection.

## Evidence

- `../../../examples/specification/run/hpack-fixture-huffman-eos-json/` and
  `../../../examples/specification/run/hpack-fixture-huffman-eos-human/`
  check the direct Huffman-EOS standard helper projection.
- `../../../examples/specification/run/hpack-fixture-huffman-non-visible-json/`
  and
  `../../../examples/specification/run/hpack-fixture-huffman-non-visible-human/`
  check the direct Huffman non-visible standard helper projection.
- `../../../examples/specification/run/hpack-fixture-dynamic-index-json/` and
  `../../../examples/specification/run/hpack-fixture-dynamic-index-human/`
  check the direct dynamic-index standard helper projection.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-string-length-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-raw-string-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-padding-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-eos-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-dynamic-index-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-table-size-malformed-json/`,
  and
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-table-size-json/`
  check helper-returned JSON `details.value` shapes and focused protocol
  diagnostic projections.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-table-size-malformed-human/`
  checks the helper-returned malformed table-size update integer human
  projection.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-json/`,
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-json/`,
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-json/`,
  `../../../examples/specification/run/http2-protocol-core-hpack-raw-string-json/`,
  `../../../examples/specification/run/http2-protocol-core-hpack-string-length-json/`,
  and
  `../../../examples/specification/run/http2-protocol-core-hpack-table-size-placement-json/`
  check the HTTP/2 protocol-core paths that report HPACK fixture helper
  payloads directly.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the implemented command-facing
  behavior.
