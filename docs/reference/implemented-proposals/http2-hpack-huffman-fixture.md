# HTTP/2 HPACK Huffman Fixture

Status: implemented

This record preserves the completed narrow HPACK Huffman fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary accepts narrow Huffman-flagged
literal-without-indexing header blocks. The `0x04 0x80` fixture uses indexed
static-table name `:path`, sets the Huffman flag on a zero-length value, and
decodes the fixture value as the empty string. The
`0x01 0x8c 0xf1 0xe3 0xc2 0xe5 0xf2 0x3a 0x6b 0xa0 0xab 0x90 0xf4 0xff`
fixture uses indexed static-table name `:authority`, sets the Huffman flag on
the value bytes, and decodes `:authority: www.example.com`.

The transition returns the same immutable `HpackFixtureState` shape and
advances the decode count through the existing transition accessors.
Unsupported or malformed Huffman variants still project through
`hpack.fixture.unsupported_header_block`.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the focused `literal-path-empty-huffman` decode and a malformed Huffman
  variant that stays on the unsupported-header-block path.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS cases named `hpack-literal-huffman` and
  `hpack-literal-authority-www-example-huffman`, emits the header-block bytes
  `0480` and `018cf1e3c2e5f23a6ba0ab90f4ff`, prints the decoded `:path`
  and `:authority: www.example.com` values, and keeps malformed Huffman input
  on `hpack.fixture.unsupported_header_block`.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
