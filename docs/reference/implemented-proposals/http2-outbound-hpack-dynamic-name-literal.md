# HTTP/2 Outbound HPACK Dynamic-Name Literal

Status: implemented

This record preserves the completed outbound HPACK fixture
literal-without-indexing slice from the HTTP/2 sans-I/O protocol-core
proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case
`../../../examples/specification/run/hpack-fixture-codec-boundary/`.

## Completed Behavior

The imported HPACK fixture encoder accepts one fixture-scoped outbound
literal-without-indexing header list whose name is selected from the bounded
dynamic table and whose value is supplied as a fresh raw literal. The checked
path first encodes a literal-with-indexing `:path: /target` header list to
insert a dynamic entry. A later `:path: /fresh` fixture header list encoded
from that returned state emits the HPACK dynamic-name literal bytes
`0x0f 0x2f 0x06 "/fresh"` and returns another immutable encode state without
inserting a replacement entry. Encoding `:path: /target` from that returned
state still emits the dynamic indexed byte `0xbe`.

Encoding the dynamic-name literal fixture without a matching dynamic-table
name remains a focused HPACK fixture failure. The failure stays at the HPACK
fixture boundary with expected fixture
`fixture outbound dynamic-name indexed literal` instead of falling back to a
generic string or HTTP/2 protocol diagnostic.
If the dynamic-table name is present but the fresh value is not supported by
the raw string fixture encoder, the failure remains the focused
`fixture raw string encoding` case.

This slice does not implement full HPACK compression, general indexed-name
selection, Huffman expansion beyond the existing fixture encoder support,
socket integration, or production HTTP/2 behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the stateful insertion, the outbound dynamic-name literal bytes, explicit
  returned encode-state reuse, retained dynamic-indexed `0xbe` encoding, and
  the empty-state dynamic-name fixture failure plus the unsupported fresh
  value failure.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented fixture boundary and route readers to the checked
  example.
