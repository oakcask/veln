# HTTP/2 Outbound HPACK Dynamic-Name Never-Indexed Literal

Status: implemented

This record preserves the completed outbound HPACK fixture
dynamic-name literal-never-indexed slice from the HTTP/2 sans-I/O
protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture encoder accepts one fixture-scoped outbound
literal-never-indexed header list whose name is selected from the bounded
dynamic table and whose value is supplied as a fresh raw visible-ASCII
literal. The checked path first encodes a literal-with-indexing
`:path: /target` header list to insert a dynamic entry. A later
`:path: /secret` fixture header list encoded from that returned state emits
the HPACK dynamic-name literal-never-indexed bytes
`0x1f 0x2f 0x07 "/secret"` and returns another immutable encode state without
inserting a replacement entry.

Encoding `:path: /target` from that returned state still emits the dynamic
indexed byte `0xbe`, proving the prior dynamic entry remains reusable. The
HTTP/2 protocol-core example feeds the returned HPACK encode state into
outbound HEADERS framing: the dynamic-name literal-never-indexed header block
is emitted as HEADERS bytes, and the retained `/target` entry is reused as
`0xbe` through a later outbound HEADERS send intent.

Encoding the dynamic-name literal-never-indexed fixture without a matching
dynamic-table name remains a focused HPACK fixture failure. If the
dynamic-table name is present but the fresh value is not supported by the raw
string fixture encoder, the failure remains the focused
`fixture raw string encoding` case.

This slice does not implement full HPACK compression, general dynamic-table
indexing, Huffman expansion, socket behavior, or production HTTP/2 behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the stateful insertion, `0x1f 0x2f 0x07 "/secret"` dynamic-name
  literal-never-indexed bytes, retained dynamic-indexed `0xbe` encoding, and
  the empty-state dynamic-name fixture failure.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  returned state through outbound HEADERS framing and retained dynamic-index
  reuse through a later outbound HEADERS send intent.
- `../../specification/execution.md` summarizes the implemented fixture
  boundary and routes readers to the checked examples.
