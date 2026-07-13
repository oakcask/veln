# HTTP/2 Peer Stream ID Monotonicity

Status: implemented

This record preserves the completed peer-created stream id monotonicity slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The ordinary-source receive state carries the greatest peer-created stream id
admitted by initial HEADERS. The first valid idle peer-created stream sets the
maximum; every later new idle peer-created stream must use a greater id.
Closed and reset lifecycle states do not erase the connection-wide maximum.

A lower or previously admitted id returns typed `InvalidStreamId` with the
stable `http2.protocol.invalid_stream_id` diagnostic id, an idle-stream fact,
the required increasing-id domain, server endpoint role, and
`peer_created_stream_ids_increase` provenance. Rejection leaves the offending
frame unconsumed and leaves the next decode offset, HPACK state, shutdown
state, stream lifecycle, and greatest admitted id unchanged.

Stream-id parity and connection-stream validation still run first.
Continuation ownership, known-stream lifecycle failures, GOAWAY boundaries,
and the concurrent-stream receive limit retain their existing classification
and precedence.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks increasing
  ids, a lower id after a higher admission, a previously admitted id after its
  active slot is gone, and lower ids after closed and reset predecessors. Its
  executable assertions check the exact retained frame bytes, unchanged decode
  progress, complete carried HPACK dynamic entries, graceful-shutdown state,
  lifecycle state, and maximum-id state. They also fix the precedence of
  parity, connection-stream, continuation, known-stream, GOAWAY, and
  concurrent-stream-limit failures by diagnostic id and rule provenance.
- `../../../examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-human/`
  checks the human required-domain, endpoint, state, preview, and provenance
  notes.
- `../../../examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-json/`
  checks the matching structured protocol diagnostic and result payload.
