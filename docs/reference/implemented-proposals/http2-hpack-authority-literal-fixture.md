# HTTP/2 HPACK Authority Literal Fixture

Status: implemented

This record preserves the completed `:authority` literal fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The HTTP/2 protocol-core fixture HPACK boundary accepts one no-Huffman
literal-without-indexing header block whose indexed static-table name is
`:authority` and whose raw value is `example.com`. The decoded header list
exposes that fixture as ordinary header-list data:
`HpackHeader(":authority", "example.com")`, followed by the existing
`HpackHeader(":fixture", "literal-without-indexing")` marker.

The transition advances the immutable `HpackFixtureState` through the same
path as the other literal-without-indexing fixtures. Unsupported or malformed
literal variants still project through
`hpack.fixture.unsupported_header_block`.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` includes the
  completed HEADERS frame case named `hpack-literal-authority-example`, checks
  the emitted header-block bytes `010b6578616d706c652e636f6d`, and prints the
  decoded `:authority` header value.
- `../../specification/execution.md` states that the HTTP/2 protocol-core
  HPACK fixture boundary accepts no-Huffman literal-without-indexing fixtures
  for supported static-table names including `:authority`, with the fixed
  `example.com` value.
- `../../specification/examples.md` routes the same executable example and
  summarizes the accepted literal-without-indexing fixture set.
