# HTTP/2 HPACK Dynamic Raw Literal-Name Huffman Values

Status: implemented

This record preserves the completed source-visible raw literal-name
Huffman-value receive slice from the HTTP/2 sans-I/O protocol-core proposal.
Current behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/commands.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The `hpack_dynamic_core` boundary accepts raw literal-name header fields whose
field name is a raw visible-ASCII string and whose field value is a bounded
Huffman-marked string accepted by the existing checked HPACK Huffman boundary.
The accepted forms are:

- literal-without-indexing with a raw literal name and Huffman-marked value
- literal-with-indexing with a raw literal name and Huffman-marked value
- literal-never-indexed with a raw literal name and Huffman-marked value

The slice reuses the existing HPACK integer and checked Huffman helpers. It
does not introduce a second Huffman parser. The literal-with-indexing form
inserts the decoded name/value pair into the immutable dynamic-core state; the
literal-without-indexing and literal-never-indexed forms advance the decode
count without mutating the dynamic table. A following `0xbe` dynamic indexed
field can reuse the entry inserted by the literal-with-indexing form.

Completed HTTP/2 HEADERS and final CONTINUATION decoding route accepted raw
literal-name Huffman-value fields through this source-visible boundary before
fixture fallback. Unsupported or malformed Huffman values keep the existing
focused fixture diagnostics such as `hpack.fixture.malformed_huffman_padding`.

Huffman-marked raw literal names, full HPACK compression, unbounded
dynamic-table behavior, and unrelated stream-state behavior remain outside
this narrow receive slice.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  standalone `hpack_dynamic_core` raw literal-name Huffman values for all
  three indexing forms, dynamic-table mutation only for
  literal-with-indexing, dynamic indexed reuse through `0xbe`, and a rejected
  malformed Huffman value at the dynamic-core boundary.
- Historical aggregate evidence checks completed
  HEADERS and final CONTINUATION routing for a raw literal-name Huffman value,
  dynamic indexed reuse of the inserted entry, and focused malformed Huffman
  padding projection.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/commands.md` summarize the current behavior and route
  readers to the executable evidence.
