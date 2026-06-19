# HTTP/2 HPACK Huffman Fixture

Status: implemented

This record preserves the completed narrow HPACK Huffman fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

The later
[HPACK string literal fixture](http2-hpack-string-literal-fixture.md) record
preserves the slice that routes both raw and Huffman-marked literal values
through one string literal decoder.

## Completed Behavior

The imported HPACK fixture boundary accepts Huffman-marked
literal-without-indexing header values by scanning fixture-supported HPACK
static Huffman symbols into decoded visible-ASCII bytes rather than by
matching a fixed decoded-value allowlist. Checked values include `0x04 0x80`
for zero-length `:path`, `0x04 0x83 0x49 0x50 0x9f` for `:path: test`,
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
  `literal-path-test-huffman` decodes, the existing
  `literal-scheme-https-huffman` decode, plus malformed Huffman padding that
  stays on the unsupported-header-block path.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS cases named `hpack-literal-huffman`,
  `hpack-literal-test-huffman`, and `hpack-literal-scheme-https-huffman`,
  emits the header-block bytes `0480`, `048349509f`, and `06849d29ad1f`,
  prints the decoded `:path`, `:path: test`, and `:scheme: https` values,
  and keeps malformed Huffman padding on
  `hpack.fixture.unsupported_header_block`. The broader HTTP/2 case also
  keeps the `:authority: www.example.com` Huffman fixture covered.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
