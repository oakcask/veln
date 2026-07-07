# Binary Data HPACK Static Index Byte Preview Diagnostics

Status: implemented

This record preserves the completed HPACK static index protocol-facing byte
diagnostic slice from the binary data standard-library proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and
the checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Unsupported HPACK static indexes now project as protocol-facing byte
diagnostics for both fixture-owned and source-visible static-decoder paths.
The fixture-owned path uses `hpack.fixture.unsupported_static_index`, and the
source-visible static decoder uses `hpack.static.unsupported_index`.

Both diagnostics keep the focused human primary message on the unsupported
static index fact. Byte offset, observed header-block size, observed first
byte, expected fixture or header, codec module, and bounded nearby-byte
preview stay in notes or structured protocol-diagnostic fields.

`veln run --json` projects the same bounded `byte_preview` object shape used
by the other protocol-owned byte diagnostics: preview encoding, hex data,
preview byte count, total byte count, and truncation status. The direct
runtime-helper payload path and the protocol-core HPACK failure projection path
both use the same public fields.

This slice does not add full HPACK compression, new dynamic-table behavior,
socket behavior, or production HTTP/2 behavior.

## Evidence

- `../../../examples/specification/run/hpack-static-index-unsupported-human/`
  and `../../../examples/specification/run/hpack-static-index-unsupported-json/`
  check the direct fixture-owned unsupported static-index human and JSON
  projections.
- `../../../examples/specification/run/hpack-static-core-index-unsupported-human/`
  and
  `../../../examples/specification/run/hpack-static-core-index-unsupported-json/`
  check the source-visible `hpack.static.unsupported_index` diagnostic
  projection.
- `../../../examples/specification/run/hpack-static-index-projection-human/`
  and `../../../examples/specification/run/hpack-static-index-projection-json/`
  check the protocol-core HPACK failure projection path.
- `../../../examples/specification/run/http2-protocol-core/` keeps the
  aggregate protocol-core route for the same `hpack.static.unsupported_index`
  failure.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the command-facing diagnostic
  projection and route readers to executable evidence.
