# HTTP/2 HPACK Huffman Fixture

Status: implemented

This record preserves the completed HPACK Huffman fixture slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
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
literal-without-indexing header values by scanning the HPACK static Huffman
table into decoded fixture strings rather than by matching a fixed
decoded-value allowlist. The checked Huffman boundary accepts visible ASCII,
the line-feed fixture value, and single-byte `hpack-byte-xx` labels for every
byte value. A later fixture slice accepts multi-byte non-visible decoded byte
strings through deterministic `hpack-bytes-xx-...-xx` labels, including
`hpack-bytes-00-ff` for decoded bytes `0x00 0xff`. Checked values include
`0x04 0x80`
for zero-length `:path`, `0x04 0x83 0x49 0x50 0x9f` for `:path: test`,
`0x04 0x84 0xff 0xff 0xff 0xf3` for `:path` line feed,
`0x04 0x82 0xff 0xc7` for `:path` single NUL,
`0x04 0x84 0xff 0xff 0xfb 0xbf` for `:path` `hpack-byte-ff`,
`0x06 0x84 0x9d 0x29 0xad 0x1f` for `:scheme: https`,
`0x08 0x82 0x10 0x01` for `:status: 200`, and
`0x01 0x8c 0xf1 0xe3 0xc2 0xe5 0xf2 0x3a 0x6b 0xa0 0xab 0x90 0xf4 0xff`
for `:authority: www.example.com`.

The HTTP/2 protocol-core fixture feeds those decoded header fields through the
same completed HEADERS and final CONTINUATION paths as raw string literals, so
header-list validation and protocol-state projection run after the HPACK
fixture decoder has produced ordinary header-list data. The checked invalid
Huffman cases exercise real table decode failures: malformed EOS-prefix
padding and EOS decoded as a data symbol stay on focused HPACK fixture
diagnostics rather than widening the boundary to full HPACK compression.

The transition returns the same immutable `HpackFixtureState` shape and
advances the decode count through the existing transition accessors.
Huffman EOS stays outside full HPACK support but projects through the focused
`hpack.fixture.huffman_eos_symbol` diagnostic. The later
[HPACK malformed Huffman padding diagnostic](http2-hpack-huffman-padding-diagnostic.md)
record preserves the focused diagnostic id for malformed Huffman padding, and
the later
[HPACK multi-byte non-visible fixture](http2-hpack-multibyte-non-visible-fixture.md)
record preserves the accepted multi-byte label form.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the focused `literal-path-empty-huffman` and
  `literal-path-test-huffman`, `literal-path-linefeed-huffman`, and
  `literal-path-ff-huffman` decodes, the existing
  `literal-scheme-https-huffman` decode, the
  `literal-status-200-huffman` decode, plus malformed Huffman padding that
  was later split into a focused diagnostic path.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS cases named `hpack-literal-huffman`,
  `hpack-literal-test-huffman`, `hpack-literal-scheme-https-huffman`, and
  `hpack-literal-status-200-huffman`, emits the header-block bytes `0480`,
  `048349509f`, `0484fffffff3`, `06849d29ad1f`, and `08821001`, prints the
  decoded `:path`, `:path: test`, `:path` line feed, `:scheme: https`, and `:status: 200` values, and keeps
  the focused malformed-padding diagnostic covered through the later
  implemented record. The broader HTTP/2 case also keeps the
  `:authority: www.example.com` Huffman fixture covered and checks
  `:status: 200` through a final CONTINUATION path. It also checks focused
  Huffman EOS through a final CONTINUATION path and focused non-visible
  decoded bytes through a completed HEADERS path.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-human/`,
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-json/`,
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-human/`,
  and
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-json/`
  check the human and JSON command-output projection for the focused EOS and
  non-visible decoded-byte diagnostics with stable preview fields.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
