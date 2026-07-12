# HTTP/2 Outbound HPACK Huffman Literal Names

Status: implemented

This record preserves the completed outbound Huffman literal-name slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The source-visible HPACK fixture encoder accepts checked new literal header
names from the existing Huffman encoder for literal-without-indexing,
literal-with-indexing, and literal-never-indexed. The checked `test` name
emits the Huffman string literal `0x83 0x49 0x50 0x9f` after each form's
zero name-index prefix.

Each form composes that encoded name with both raw and Huffman literal values.
The raw `ok` value produces header blocks beginning with `0x00`, `0x40`, or
`0x10` and ending with `0x02 "ok"`. The Huffman `test` value uses
`0x83 0x49 0x50 0x9f` for both the name and value payloads.

Only literal-with-indexing inserts the decoded name/value pair into the
immutable bounded table. A later header list reuses an inserted `test: ok` or
`test: test` entry as `0xbe`. The other two forms return states with an empty
dynamic table. Unsupported Huffman-name input returns a focused fixture encode
failure without a header block; reusing the original carried state after that
failure still resolves its prior entry.

The HTTP/2 aggregate example routes the accepted indexed `test: ok` block
through outbound HEADERS, observes the returned encode state, and routes a
later `0xbe` reuse through HEADERS. This slice adds no source syntax, Huffman
table entries, transport behavior, full compression policy, or unbounded
dynamic-table behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  exact bytes for all three forms with raw and Huffman values, empty-table
  retention for non-indexing forms, indexed insertion and reuse, and failure
  preservation.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  accepted indexed header block and its dynamic-indexed reuse after outbound
  HEADERS framing, including observable returned state.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize current behavior and route readers to executable evidence.
