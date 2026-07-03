# HTTP/2 Outbound PRIORITY GOAWAY Boundary

Status: implemented

This record preserves the completed outbound `PRIORITY` post-GOAWAY
send-intent boundary from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md` and the checked
executable case `../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

Outbound `PRIORITY` send-intents observe recorded graceful-shutdown state after
stream id domain, target stream state, mismatched stream, and priority
self-dependency facts have been checked, but before frame header encoding,
priority payload encoding, output chunk emission, or stream state mutation.

After receiving GOAWAY, `PRIORITY` for an open stream id greater than the
received last stream id is rejected with
`http2.protocol.stream_after_goaway`. `PRIORITY` for the stream at the
recorded boundary remains accepted and emits the normal `PRIORITY` frame
bytes.

After locally sending GOAWAY, `PRIORITY` follows the same boundary rule.
`PRIORITY` for the sent boundary remains accepted, while an above-boundary
stream is rejected with the same structured shutdown state, endpoint role, and
rule provenance used by other outbound post-GOAWAY send-intents.

Rejected above-boundary `PRIORITY` emits no output bytes and does not encode
the frame header or priority payload. Stream id zero, missing-stream,
closed-stream, reset-stream, mismatched-stream, self-dependency, and ordinary
schema encode failures keep their narrower existing facts before or instead
of the GOAWAY boundary.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  outbound `PRIORITY` at received and locally sent GOAWAY boundaries.
- The same checked case pins above-boundary outbound `PRIORITY` after
  received and locally sent GOAWAY as no-output
  `http2.protocol.stream_after_goaway` rejections.
- The same checked case verifies that priority self-dependency keeps the
  narrower `http2.protocol.invalid_priority_dependency` rejection when the
  stream is also above the recorded GOAWAY boundary.
- `../../specification/execution.md` and
  `../../specification/run-json.md` summarize the current behavior and route
  readers to the checked executable example.
