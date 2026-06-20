# HTTP/2 HPACK String Literal Fixture

Status: implemented

This record preserves the completed HPACK string literal fixture slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary decodes literal header values through one
HPACK string literal helper for the supported static-table names
`:authority`, `:method`, `:path`, `:scheme`, and `:status`. The helper accepts
visible-ASCII raw string literals and Huffman-marked values decoded by
the HPACK static Huffman table. It also accepts the fixture-boundary
string-length integer continuation form for supported literal names, covering
one continuation byte after a saturated seven-bit string-length prefix for
long raw values, including raw values past the former checked 128-byte decode
boundary, and a deterministic long Huffman-marked value.
The same fixture module also exposes a raw string-literal encoder for values
accepted by `byte_chunk_from_visible_ascii_string`. It emits HPACK string
literal bytes with the Huffman flag cleared, including the same
one-continuation length boundary for the long raw `a` fixture, and keeps
non-visible input on the fixture-owned unsupported-header-block failure path.

The shared decoder is used by literal-without-indexing,
literal-with-indexing, and literal-never-indexed header blocks.
Literal-with-indexing returns the next immutable `HpackFixtureState` with the
inserted dynamic entry, so later indexed lookup and fixture table-size
eviction keep the same state behavior as the earlier dynamic-table slice.
Literal-never-indexed advances the fixture decode count without inserting a
dynamic-table entry.

Malformed Huffman padding, malformed string lengths including non-terminating
string-length continuations, non-visible raw bytes, unsupported names,
Huffman EOS, and Huffman strings whose decoded bytes are not visible ASCII
continue to project through
`hpack.fixture.unsupported_header_block`; this slice does not introduce a
narrower diagnostic.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  raw and Huffman-marked literal-without-indexing values, Huffman-marked
  `:path: test`, Huffman-marked `:status: 200`, Huffman-marked
  literal-with-indexing `:method: PUT`, raw literal-with-indexing
  `:authority`, Huffman-marked literal-with-indexing `:scheme: https`, raw
  literal-with-indexing `:status`, raw literal-never-indexed `:authority`,
  Huffman-marked literal-never-indexed `:scheme: https`, long raw and
  Huffman-marked string-length continuation fixtures through all three
  literal forms, 129-byte raw values through all three literal forms,
  malformed string-length continuation, malformed Huffman
  padding, short and long raw string-literal encoding, unsupported and
  non-visible raw string-literal encoding failures, dynamic-table behavior
  after literal-with-indexing insertions, and no dynamic-table insertion after
  literal-never-indexed decoding.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  string literal cases through completed HEADERS and final CONTINUATION paths.
  Long valid values reach the HPACK boundary and then the local header-list
  receive-limit check on the protocol-core path, including a 129-byte raw
  final CONTINUATION case, while malformed fixture inputs preserve the
  existing `hpack.fixture.unsupported_header_block` diagnostic path.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented fixture boundary and route readers to the checked
  examples.
