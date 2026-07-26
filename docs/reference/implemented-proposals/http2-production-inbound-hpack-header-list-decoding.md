# HTTP/2 Production Inbound HPACK Header-List Decoding

Status: implemented

This record preserves the completed production inbound header-list slice from
the HTTP/2 sans-I/O protocol-core proposal. Current reusable behavior is
specified by `../../specification/http2.md` and the checked executable case
under `../../../examples/specification/run/hpack-header-block-decoding/`.

## Completed Behavior

The public `std::http2::hpack` decoder returns a recursive ordinary Veln
header-field value for an arbitrary finite sequence of supported HPACK indexed
and literal representations. Field boundaries use the shared HPACK integer and
string length decoders, including checked Huffman strings, rather than another
fixed header-count shape.

The immutable decode state applies legal leading table-size updates, indexed
literal insertions, and byte-accounted evictions in wire order. A later block
can resolve retained dynamic entries. If any field fails, the public result
contains neither a partial header list nor a next state, so the caller retains
the input state.

The reusable facade is independent of the monolithic protocol fixture.
Complete inbound request, response, and trailer HEADERS blocks and final
CONTINUATION assembly remain covered by the protocol-core evidence while the
standard decoder owns the codec and immutable-state invariants.

## Evidence

- `../../../crates/veln-stdlib/veln/http2/hpack_test.veln` checks empty,
  update-only, and mixed blocks; every literal representation; field order and
  list boundaries; exact non-visible value octets; in-block and cross-decode
  dynamic references; leading table-size updates; nested failure families; and
  input-state preservation.
- `../../../examples/specification/run/hpack-header-block-decoding/` records
  public facade result values and representative typed failures.
- `../../../examples/specification/run/http2-protocol-core/` continues to check a
  five-field mixed static, literal-with-indexing, and dynamic-indexed block;
  retained-entry reuse in a later block; byte-accounted eviction after a
  table-size reduction; complete request, response, and trailer routing; split
  request and response CONTINUATION assembly; invalid later request, response,
  and trailer fields; state reuse through the HTTP/2 boundary; and exact input
  HPACK state retention after a late validation failure.
- `../../specification/http2.md` specifies the reusable standard facade.
