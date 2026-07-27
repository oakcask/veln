# HTTP/2 Outbound WINDOW_UPDATE GOAWAY Boundary

Status: implemented

This record preserves the completed outbound stream-level `WINDOW_UPDATE`
post-GOAWAY send-intent boundary from the HTTP/2 sans-I/O protocol-core
proposal. Current behavior is specified by `../../specification/execution.md`
and the checked executable case
historical aggregate evidence.

## Completed Behavior

Outbound stream-level `WINDOW_UPDATE` receive-credit intents observe recorded
graceful-shutdown state after the target stream has been confirmed as the
currently open stream, but before increment encoding, receive-credit changes,
or output bytes.

After receiving GOAWAY, a stream-level `WINDOW_UPDATE` for an open stream id
greater than the received last stream id is rejected with
`http2.protocol.stream_after_goaway`. A stream at the recorded boundary
remains accepted and emits the normal `WINDOW_UPDATE` frame bytes.

After locally sending GOAWAY, stream-level `WINDOW_UPDATE` follows the same
boundary rule. Connection-level outbound `WINDOW_UPDATE` remains valid after
GOAWAY, subject to the existing increment and receive-window checks.

Rejected above-boundary `WINDOW_UPDATE` emits no output bytes and does not
change connection or stream receive credit. Stream id zero, idle-stream,
closed-stream, reset-stream, mismatched-stream, increment range, and
receive-window overflow cases keep their narrower existing failures before or
instead of the GOAWAY boundary.

## Evidence

- Historical aggregate evidence checks accepted
  stream-level outbound `WINDOW_UPDATE` at the received GOAWAY boundary and
  at the locally sent GOAWAY boundary.
- The same checked case pins above-boundary stream-level outbound
  `WINDOW_UPDATE` after received and locally sent GOAWAY as no-output
  `http2.protocol.stream_after_goaway` rejections.
- The same checked case keeps connection-level outbound `WINDOW_UPDATE`
  accepted after GOAWAY and preserves the existing stream id, stream-state,
  increment range, overflow, and encode-error coverage.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the current behavior and route readers to the checked executable
  example.
