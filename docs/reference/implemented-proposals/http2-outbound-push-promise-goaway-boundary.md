# HTTP/2 Outbound PUSH_PROMISE GOAWAY Boundary

Status: implemented

This record preserves the completed server-side outbound `PUSH_PROMISE`
post-GOAWAY send-intent boundary from the HTTP/2 sans-I/O protocol-core
proposal. Current behavior is specified by `../../specification/execution.md`
and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

Outbound `PUSH_PROMISE` send-intents observe recorded graceful-shutdown state
after stream id domain, peer disable-push, associated stream state, and
promised stream id facts have been checked, but before HPACK fixture encoding,
generated promised-stream payload encoding, frame splitting, or output chunk
emission.

After receiving GOAWAY, `PUSH_PROMISE` for an open associated stream id greater
than the received last stream id is rejected with
`http2.protocol.stream_after_goaway`. `PUSH_PROMISE` for the stream at the
recorded boundary remains accepted and emits the normal `PUSH_PROMISE` frame
bytes.

After locally sending GOAWAY, `PUSH_PROMISE` for an open associated stream id
greater than the sent last stream id follows the same rejection shape, with
shutdown state, endpoint role, and rule provenance projected through the same
protocol diagnostic path used by outbound HEADERS and DATA. `PUSH_PROMISE` for
the sent boundary remains accepted.

Rejected above-boundary `PUSH_PROMISE` emits no output bytes and does not run
the HPACK fixture encoder. Missing-stream, closed-stream, reset-stream,
disabled-push, stream-id-domain, promised-stream-id, and ordinary HPACK fixture
failure cases keep their narrower existing facts before the GOAWAY boundary is
considered.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  outbound `PUSH_PROMISE` at the received GOAWAY boundary and at the locally
  sent GOAWAY boundary.
- The same checked case pins above-boundary `PUSH_PROMISE` after received and
  locally sent GOAWAY as no-output `http2.protocol.stream_after_goaway`
  rejections.
- The same checked case verifies that a shutdown-blocked HPACK-list
  `PUSH_PROMISE` rejects before the HPACK fixture encoder reports an otherwise
  unsupported header-list encode failure.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/commands.md` summarize the current behavior and route
  readers to the checked executable example.
