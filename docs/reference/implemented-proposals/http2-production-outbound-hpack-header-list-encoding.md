# HTTP/2 Production Outbound HPACK Header-List Encoding

Status: implemented

This record preserves the completed production outbound header-list slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case under
historical aggregate evidence.

## Completed Behavior

The source-visible encoder accepts a recursive ordinary Veln value containing
any finite ordered list of already-validated header name/value pairs. It does
not inspect fixture labels or assume a fixed retained-entry count. For every
field it uses the shared HPACK integer, raw string, literal, static-table, and
dynamic-table helpers to select exact static indexed, exact dynamic indexed,
static-name literal, dynamic-name literal, or new-name literal in that order.

The returned immutable state retains byte-accounted dynamic entries across
successive blocks. Literal-with-indexing insertion observes the active
peer-advertised capacity, evicts oldest entries through the existing
accounting rule, and honors table-size updates including zero capacity. A peer
capacity reduction adds the required table-size update at the start of the
next ordered block and evicts state before field encoding.

Outbound request and response HEADERS and server-side `PUSH_PROMISE` feed the
encoded block into the existing maximum-frame-size splitting paths. Rejected
input returns no encoded transition, so callers cannot emit partial frames or
commit intermediate HPACK, stream, flow-control, or shutdown state.

## Evidence

- Historical aggregate evidence checks a
  four-field block, all five representation choices across carried blocks,
  dynamic reuse, reduced-capacity eviction, automatic peer-capacity reduction,
  zero-capacity behavior, request and response HEADERS splitting, server-side
  `PUSH_PROMISE` splitting, and state reuse after an invalid later field.
- `../../specification/execution.md` summarizes the current encoder boundary
  and routes readers to the executable evidence.
