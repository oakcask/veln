# HTTP/2 HPACK No-Huffman Raw Literal Fixture

Status: implemented

This record preserves the completed no-Huffman raw literal fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable case
historical aggregate evidence.

## Completed Behavior

The HTTP/2 protocol-core fixture HPACK boundary accepts no-Huffman
literal-without-indexing header blocks whose indexed static-table name is one
of the supported request pseudo-header names, whose one-byte value length is
not Huffman-marked, whose value length is within the small fixture bound, and
whose raw value bytes are all visible ASCII. The decoded header list exposes
the selected name and raw value as ordinary header-list data, followed by the
existing `HpackHeader(":fixture", "literal-without-indexing")` marker.

The later
[HPACK string literal fixture](http2-hpack-string-literal-fixture.md) record
preserves the slice that shares this raw-value path with Huffman-marked
literal values and literal-with-indexing header blocks.

The transition advances the immutable `HpackFixtureState` through the same
path as the other literal-without-indexing fixtures. Unsupported literal
variants still project through `hpack.fixture.unsupported_header_block`.
Malformed or non-visible raw values on supported literal names use focused
HPACK fixture diagnostics.

## Evidence

- Historical aggregate evidence includes the
  completed HEADERS frame case named `hpack-literal-authority-raw` and the
  final CONTINUATION case named
  `hpack-literal-authority-raw-continuation`, checks the emitted header-block
  bytes `01086162632e74657374`, and prints the decoded `:authority` header
  value.
- The same executable case includes `hpack-literal-non-visible`, which rejects
  a no-Huffman literal value containing a non-visible byte through focused
  HPACK fixture diagnostics.
- `../../specification/execution.md` states that the HTTP/2 protocol-core
  HPACK fixture boundary accepts no-Huffman literal-without-indexing fixtures
  for supported static-table names when the raw value bytes are all visible
  ASCII.
- `../../specification/examples.md` routes the same executable example and
  summarizes the accepted literal-without-indexing raw-value fixture set.
