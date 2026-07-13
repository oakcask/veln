# HTTP/2 Production Inbound HPACK Header-List Decoding

Status: implemented

This record preserves the completed production inbound header-list slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case under
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The source-visible decoder returns a recursive ordinary Veln header-field value
for an arbitrary finite sequence of already-supported HPACK indexed and literal
representations. Field boundaries use the shared HPACK integer and string
length decoders, including checked Huffman strings, rather than another fixed
header-count shape.

The immutable decode state applies legal leading table-size updates, indexed
literal insertions, and byte-accounted evictions in wire order. A later block
can resolve retained dynamic entries. If any field fails, the public result
contains neither a partial header list nor a next state, so the caller retains
the input state.

Complete inbound request, response, and trailer HEADERS blocks and final
CONTINUATION assembly carry production-shaped lists through the recursive
boundary. Request, response, trailer, content-length, and stream-state helpers
inspect the wire-order fields while keeping their established validation order
and diagnostic identifiers.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks a
  five-field mixed static, literal-with-indexing, and dynamic-indexed block;
  retained-entry reuse in a later block; byte-accounted eviction after a
  table-size reduction; complete request, response, and trailer routing; split
  request and response CONTINUATION assembly; invalid later request, response,
  and trailer fields; state reuse through the HTTP/2 boundary; and exact input
  HPACK state retention after a late validation failure.
- `../../specification/execution.md` summarizes the current inbound boundary
  and routes readers to the executable evidence.
