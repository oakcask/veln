# Runtime Diagnostic HPACK Fixture Payloads

Status: implemented

This record preserves the completed HPACK fixture runtime diagnostic payload
slice from the runtime diagnostic payload proposal. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/test-json.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

HPACK fixture result failures can now be carried as ordinary
`Err(RuntimeDiagnostic(...))` values instead of depending only on
backend-local side-table registrations keyed by rendered error messages. The
common fixture detail constructor carries the diagnostic id, message, byte
offset, observed header-block size, observed first byte, expected fixture,
codec module, and bounded header-block preview. Dynamic-index and table-size
update placement diagnostics use dedicated detail constructors for the extra
facts those projections need.

The implemented HPACK fixture payload ids are:

- `hpack.fixture.unsupported_header_block`
- `hpack.fixture.malformed_string_length`
- `hpack.fixture.malformed_raw_string_value`
- `hpack.fixture.malformed_huffman_padding`
- `hpack.fixture.huffman_eos_symbol`
- `hpack.fixture.huffman_non_visible_value`
- `hpack.fixture.dynamic_index_out_of_range`
- `hpack.fixture.table_size_update_not_at_start`

Command projection keeps the rendered `RuntimeDiagnostic(...)` value in the
result-failure details and projects the contained payload into the existing
human HPACK fixture diagnostic and `details.protocol_diagnostic` JSON shapes.
Plain `Err(value)` values remain ordinary result failures and do not opt into
runtime diagnostic projection.

This slice deliberately keeps legacy backend side-table support for existing
fixture, value, protocol, HTTP/2, and generated-schema helpers while the
remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-human/` and
  `../../../examples/specification/run/hpack-fixture-codec-json/` check the
  source-visible unsupported-header-block projection.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-string-length-human/`
  and
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-string-length-json/`
  check the source-visible malformed-string-length projection and returned
  value shape.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-raw-string-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-padding-json/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-eos-json/`,
  and
  `../../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-non-visible-human/`
  check the additional common HPACK fixture payload ids.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-dynamic-index-json/`
  checks the dedicated dynamic-index payload constructor and JSON projection.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-table-size-human/`
  checks the dedicated table-size update placement payload constructor and
  human projection.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and `../../specification/test-json.md`
  summarize the implemented behavior and route readers to executable
  evidence.
