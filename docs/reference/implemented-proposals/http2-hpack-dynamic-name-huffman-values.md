# HTTP/2 HPACK Dynamic-Name Huffman Values

Status: implemented

This record preserves the completed source-visible dynamic-name
Huffman-value receive slice from the HTTP/2 sans-I/O protocol-core proposal.
Current behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/commands.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The `hpack_dynamic_core` boundary accepts literal header fields whose field
name is resolved from the carried bounded dynamic table and whose field value
is a bounded Huffman-marked string accepted by the checked HPACK Huffman
boundary. The accepted forms are:

- literal-without-indexing with a dynamic-table name and Huffman-marked value
- literal-with-indexing with a dynamic-table name and Huffman-marked value
- literal-never-indexed with a dynamic-table name and Huffman-marked value

The slice reuses the existing HPACK integer and checked Huffman helpers. It
does not introduce a second Huffman parser. The literal-with-indexing form
inserts the decoded name/value pair into the immutable dynamic-core state; the
literal-without-indexing and literal-never-indexed forms advance the decode
count without mutating the dynamic table. A following `0xbe` dynamic indexed
field can reuse the entry inserted by the literal-with-indexing form, while
the non-inserting forms keep the earlier dynamic entry reusable.

Completed HTTP/2 HEADERS and final CONTINUATION decoding route accepted
dynamic-name Huffman-value fields through this source-visible boundary before
fixture fallback. Malformed Huffman-marked values keep the existing focused
fixture fallback shape used by raw literal-name Huffman values.

Full HPACK compression, unbounded dynamic-table behavior, and unrelated
header validation remain outside this receive slice.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  standalone `hpack_dynamic_core` dynamic-name Huffman values for all three
  indexing forms, dynamic-table mutation only for literal-with-indexing,
  retained dynamic-name reuse through `0xbe`, and a rejected malformed
  Huffman value at the dynamic-core boundary.
- `../../../examples/specification/run/http2-protocol-core/` checks completed
  HEADERS and final CONTINUATION routing for a dynamic-name Huffman value, plus
  dynamic indexed reuse of the inserted entry after each route.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/commands.md` summarize the current behavior and route
  readers to the executable evidence.
