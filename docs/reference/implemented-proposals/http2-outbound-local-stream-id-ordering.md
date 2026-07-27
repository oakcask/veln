# HTTP/2 Outbound Local Stream ID Ordering

Status: implemented

This record preserves the completed client-side outbound HEADERS local-stream
admission and identifier-ordering slice. Current behavior is specified by
`../../specification/execution.md`, `../../specification/commands.md`,
`../../specification/run-json.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The outbound HEADERS connection state accepts an idle nonzero
client-initiated stream id as a new local stream and retains the greatest id
whose complete send intent was accepted. First and increasing ids advance the
high-water value. Closing or resetting the latest stream does not lower it.
HEADERS on a tracked open stream keeps the existing lifecycle path without a
new-stream ordering check, and server endpoints cannot start regular streams.

Reused and lower new ids use
`http2.protocol.peer_stream_id_not_increasing` with the attempted id, previous
high-water value, client endpoint role, active state, and rule provenance.
Stream-id domain, endpoint role, GOAWAY, peer concurrent-stream limit, HPACK,
frame-size, splitting, and generated encoding checks retain their focused
paths. Ordering state advances only after complete single-frame or split
HEADERS/CONTINUATION encoding succeeds. Rejection emits no output chunk and
preserves HPACK, receive-credit, peer-settings, shutdown, lifecycle, and
ordering state. A rejected higher id remains eligible for corrected retry.

## Evidence

- Historical aggregate evidence checks client and
  server roles; first, increasing, repeated, and lower ids; retention after
  close and reset; existing-stream reuse; validation precedence; retry after
  rejection; HPACK rollback; split output; and connection-state preservation.
- `../../../examples/specification/run/http2-protocol-core-outbound-local-stream-id-ordering-human/`
  checks the focused primary message and related client notes.
- `../../../examples/specification/run/http2-protocol-core-outbound-local-stream-id-ordering-json/`
  checks the source-visible runtime value and structured JSON fields.

## Non-Goals

Automatic stream-id allocation, server-initiated regular streams,
`PUSH_PROMISE` promised-stream ordering, transport behavior, full HPACK
compression, and a generic stream-id allocator remain outside this completed
slice.
