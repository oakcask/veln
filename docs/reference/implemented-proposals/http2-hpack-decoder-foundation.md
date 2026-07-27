# HTTP/2 HPACK Decoder Foundation

Status: implemented

This record preserves the completed shared decoder foundation slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
historical aggregate evidence.

## Completed Behavior

The imported HPACK fixture module now routes its supported HPACK-prefixed
integer forms through one bounded decoder foundation. Table-size updates,
dynamic-name indexes, and string literal lengths share the same saturated
prefix and continuation-byte handling before each caller applies the relevant
fixture policy.

The same fixture module routes supported HPACK string literal values through
one literal decoder for static names and dynamic names across
literal-without-indexing, literal-with-indexing, and literal-never-indexed
forms. Raw values remain limited to visible ASCII, and Huffman-marked values
remain limited to checked fixture strings decoded from the HPACK static
Huffman table.
Literal-with-indexing still inserts a bounded dynamic-table entry, while
literal-without-indexing and literal-never-indexed advance decode state
without inserting replacement entries.

Unsupported fixture input remains `hpack.fixture.unsupported_header_block`.
Malformed string lengths, malformed raw string values on supported literal
names, and malformed Huffman inputs use focused `hpack.fixture.*` diagnostics.
HTTP/2 receive-policy failures remain under `http2.peer_limit.*`.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  saturated-prefix table-size integers, dynamic-name integer continuations,
  raw string literals, Huffman-marked checked fixture string literals, and the
  non-inserting literal dynamic-name forms directly at the HPACK fixture
  boundary.
- Historical aggregate evidence checks the same
  decoder paths through completed HEADERS and final CONTINUATION processing,
  including saturated dynamic-name indexes for inserting and non-inserting
  literal forms and saturated table-size updates carried through HTTP/2
  receive policy.
