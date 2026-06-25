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
HPACK string literal helper for supported static-table names. This slice
introduced the helper for `:authority`, `:method`, `:path`, `:scheme`, and
`:status`; the later
`http2-hpack-static-name-literal-fixture.md` record extends the same helper to
ordinary static-table names accepted by the static-indexed fixture set. The
helper accepts visible-ASCII raw string literals and checked Huffman-marked
values decoded by the HPACK static Huffman table. The checked Huffman boundary
accepts visible ASCII, the line-feed fixture value, and single-byte
`hpack-byte-xx` labels for every byte value while multi-byte decoded
non-visible byte strings remain outside the supported fixture boundary.
It also accepts the fixture-boundary
string-length integer continuation form for supported literal names, covering
one continuation byte after a saturated seven-bit string-length prefix for
long raw values, including raw values past the former checked 128-byte decode
boundary, and a deterministic long Huffman-marked value.
The same fixture module also exposes a raw string-literal encoder for values
accepted by `byte_chunk_from_visible_ascii_string`. It emits HPACK string
literal bytes with the Huffman flag cleared, including the same
one-continuation length boundary for the long raw `a` fixture, and keeps
non-visible raw encoder input on the fixture-owned unsupported-header-block
failure path. The decode path keeps multi-byte non-visible Huffman fixture
values outside the accepted string boundary but reports them through
`hpack.fixture.huffman_non_visible_value`.

The shared decoder is used by literal-without-indexing,
literal-with-indexing, and literal-never-indexed header blocks.
Literal-with-indexing returns the next immutable `HpackFixtureState` with the
inserted dynamic entry, so later indexed lookup and fixture table-size
eviction keep the same state behavior as the earlier dynamic-table slice.
Literal-never-indexed advances the fixture decode count without inserting a
dynamic-table entry.
Raw new-name literal forms use the same raw string literal helper for their
field-name bytes. Lower-case token names flow into the accepted HTTP/2
header-list paths, while uppercase or token-invalid raw names fail through
the same request or trailer header-list diagnostics as indexed-name literals.

Malformed string lengths including non-terminating string-length
continuations use `hpack.fixture.malformed_string_length`. Malformed raw
string values for supported literal names, including non-visible raw bytes,
use `hpack.fixture.malformed_raw_string_value`. Unsupported names continue to
project through `hpack.fixture.unsupported_header_block`. Malformed Huffman
padding keeps the established `hpack.fixture.malformed_huffman_padding`
diagnostic. Huffman EOS and Huffman strings whose decoded bytes fall outside
the supported checked fixture string values remain unsupported but use focused HPACK
fixture diagnostics.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  raw and Huffman-marked literal-without-indexing values, non-allowlist raw
  values `:authority: odd`, `:method: raw`, and
  `:path: bot` across the three literal indexing forms,
  `:path: test`, Huffman-marked `:path` line feed, Huffman-marked
  `:method: bad` through all three literal forms, Huffman-marked `:status: 200`, Huffman-marked
  `:authority: www.example.com`, Huffman-marked
  literal-with-indexing `:method: PUT`, raw literal-with-indexing
  `:authority`, Huffman-marked literal-with-indexing `:scheme: https`, raw
  literal-with-indexing `:status`, raw literal-never-indexed `:authority`,
  Huffman-marked literal-never-indexed `:scheme: https`, long raw and
  Huffman-marked string-length continuation fixtures through all three
  literal forms, 129-byte raw values through all three literal forms,
  a checked two-NUL Huffman-marked decode fixture that reports
  `hpack.fixture.huffman_non_visible_value`, malformed string-length
  continuation, malformed Huffman padding, short and long raw string-literal
  encoding, unsupported and
  non-visible raw string-literal encoding failures, dynamic-table behavior
  after literal-with-indexing insertions, and no dynamic-table insertion after
  literal-never-indexed decoding.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  string literal cases through completed HEADERS and final CONTINUATION paths.
  Long valid values reach the HPACK boundary and then the local header-list
  receive-limit check on the protocol-core path, including a 129-byte raw
  final CONTINUATION case, while malformed string lengths, malformed raw
  string values on supported literal names, Huffman padding, EOS, and
  non-visible decoded-byte diagnostics use focused HPACK fixture ids.
- `../../../examples/specification/run/http2-protocol-core-hpack-raw-name-token-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-hpack-raw-name-uppercase-json/`
  pin raw field-name validation on the existing HTTP/2 header-list diagnostic
  style.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented fixture boundary and route readers to the checked
  examples.
