# HTTP/2 Outbound DATA GOAWAY Boundary

Status: implemented

This record preserves the completed outbound DATA post-GOAWAY send-intent
boundary from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md` and the checked executable
historical aggregate evidence.

## Completed Behavior

Outbound DATA send-intents observe recorded graceful-shutdown state after the
target stream has been confirmed as the currently open stream, but before
frame-size splitting, encode checks, or outbound credit changes.

After receiving GOAWAY, DATA for an open stream id greater than the received
last stream id is rejected with `http2.protocol.stream_after_goaway`. DATA for
the stream at the recorded boundary remains accepted and emits the normal DATA
frame bytes.

After locally sending GOAWAY, DATA for an open stream id greater than the sent
last stream id follows the same rejection shape, with shutdown state, endpoint
role, and rule provenance projected through the same protocol diagnostic path
used by outbound HEADERS. DATA for the sent boundary remains accepted.

Rejected above-boundary DATA emits no output bytes and does not consume
outbound connection or stream credit. Missing-stream, closed-stream,
reset-stream, and mismatched-stream cases keep their narrower existing
failures before the GOAWAY boundary is considered.

## Evidence

- Historical aggregate evidence checks accepted
  outbound DATA at the received GOAWAY boundary and at the locally sent GOAWAY
  boundary.
- The same checked case pins above-boundary DATA after received and locally
  sent GOAWAY as no-output `http2.protocol.stream_after_goaway` rejections.
- The same checked case keeps existing outbound DATA flow-control, frame
  splitting, PADDED DATA, local `END_STREAM`, closed-stream, reset-stream, and
  mismatched-stream coverage passing.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the current behavior and route readers to the checked executable
  example.
