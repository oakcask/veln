# HTTP/2 HPACK Huffman Fixture

Status: implemented

This record preserves the completed narrow HPACK Huffman fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary accepts narrow Huffman-marked
literal-without-indexing header values. It uses the HPACK static Huffman code
table for the fixture-supported decoded values `""`, `www.example.com`,
`https`, `/target`, and `PUT`. Checked values include `0x04 0x80` for
zero-length `:path`,
`0x06 0x84 0x9d 0x29 0xad 0x1f` for `:scheme: https`, and
`0x01 0x8c 0xf1 0xe3 0xc2 0xe5 0xf2 0x3a 0x6b 0xa0 0xab 0x90 0xf4 0xff`
for `:authority: www.example.com`.

The transition returns the same immutable `HpackFixtureState` shape and
advances the decode count through the existing transition accessors.
Unsupported symbols and malformed Huffman padding still project through
`hpack.fixture.unsupported_header_block`.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the focused `literal-path-empty-huffman` and
  `literal-scheme-https-huffman` decodes plus malformed Huffman padding that
  stays on the unsupported-header-block path.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS cases named `hpack-literal-huffman` and
  `hpack-literal-scheme-https-huffman`, emits the header-block bytes `0480`
  and `06849d29ad1f`, prints the decoded `:path` and `:scheme: https`
  values, and keeps malformed Huffman padding on
  `hpack.fixture.unsupported_header_block`. The broader HTTP/2 case also
  keeps the `:authority: www.example.com` Huffman fixture covered.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
