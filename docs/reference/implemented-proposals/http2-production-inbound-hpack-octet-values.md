# HTTP/2 Production Inbound HPACK Octet Values

Status: implemented

This record preserves the completed octet-value follow-up from the HTTP/2
sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case under
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The production inbound ordered header-list decoder retains literal values as
immutable octets. Raw and Huffman-decoded values remain exact through
literal-without-indexing, literal-with-indexing, literal-never-indexed,
static-name, dynamic-name, and dynamic-indexed paths. Dynamic-table size
accounting, insertion, eviction, and reuse use the decoded byte count and do
not substitute fixture labels for non-visible values.

Header names remain on the validated string path. Request, response, and
trailer validation converts only value-sensitive fields to checked ASCII
shapes. Ordinary field values remain opaque. Decode and validation failures
continue to expose neither partial fields nor committed HPACK or protocol
state.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks raw and
  Huffman-decoded `00ff` values, all literal indexing policies, static and
  dynamic name resolution, insertion followed by dynamic-indexed reuse,
  byte-preserving state access, complete HEADERS routing, final CONTINUATION
  assembly, and atomic late failure behavior.
- The same case retains request, response, request-trailer, and
  response-trailer coverage for `:method`, `:scheme`, `:path`, `:authority`,
  `:status`, `te`, and `content-length` validation.
- `../../specification/execution.md` summarizes the current boundary and
  routes readers to the executable evidence.
