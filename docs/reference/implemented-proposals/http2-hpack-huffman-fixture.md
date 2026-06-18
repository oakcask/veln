# HTTP/2 HPACK Huffman Fixture

Status: implemented

This record preserves the completed narrow HPACK Huffman fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary accepts one Huffman-flagged
literal-without-indexing header block, `0x04 0x80`. The indexed static-table
name is `:path`, the Huffman flag is set on a zero-length value, and the
decoded fixture value is the empty string.

The transition returns the same immutable `HpackFixtureState` shape and
advances the decode count through the existing transition accessors.
Unsupported or malformed Huffman variants still project through
`hpack.fixture.unsupported_header_block`.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the focused `literal-path-empty-huffman` decode and a malformed Huffman
  variant that stays on the unsupported-header-block path.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS case named `hpack-literal-huffman`, emits the header-block
  bytes `0480`, prints the decoded `:path` value, and keeps a malformed
  Huffman input on `hpack.fixture.unsupported_header_block`.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
