# HTTP/2 HPACK Integer Core

Status: implemented

This record preserves the completed source-visible HPACK integer core slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable case
`../../../examples/specification/run/hpack-fixture-codec-boundary/`.

## Completed Behavior

The `hpack_dynamic_core` boundary exposes a bounded HPACK integer decoder and
encoder for the prefix widths used by the checked protocol slice. The decoder
accepts saturated prefix values and continuation bytes for seven-bit indexed
header fields, five-bit dynamic table-size update values, and seven-bit string
literal lengths. The encoder emits the same bounded integer shapes for indexed
fields, table-size updates, and literal lengths.

Malformed non-terminating integer continuations now report a focused
`hpack.integer.malformed` fact from the source-visible integer core. The
payload records the byte offset, prefix width, observed byte count, observed
first byte, bounded preview count, module name, and the bounded inspected byte
preview.

This remains a deterministic specification slice. It does not add full HPACK
compression, unbounded integer growth, unbounded dynamic-table behavior, or
new protocol stream-state rules.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  source-visible integer decode for a seven-bit indexed representation, a
  five-bit table-size update representation, and a seven-bit literal-length
  representation.
- The same case checks a non-terminating continuation failure with the focused
  id, byte offset, prefix width, observed byte count, observed first byte, and
  bounded preview count, and `hpack_dynamic_core` module name.
- The same case checks source-visible integer encoding for the indexed,
  table-size update, and literal-length shapes used by the existing HPACK
  fixture/core boundaries.
