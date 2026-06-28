# HTTP/2 Client PUSH_PROMISE Receive And Promised HEADERS

Status: implemented

This record preserves the completed client-side peer-sent `PUSH_PROMISE`
receive and promised response HEADERS admission slices from the HTTP/2
sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

A client receive fixture state can mark an associated client-created stream as
open for the peer-sent `PUSH_PROMISE` boundary. On that stream, the receive
path accepts a `PUSH_PROMISE` frame when its payload starts with a nonzero
server-initiated promised stream id followed by a supported HPACK fixture
request header block.

The receive path validates the associated stream and promised stream id before
ordinary state update, strips the four-byte promised-stream field before HPACK
fixture decoding, and routes the remaining header block through the same
completed HEADERS and final CONTINUATION paths used by existing header-block
fixtures. The accepted state records the promised stream as reserved by peer.

The promised-stream lifecycle slice accepts the first valid response HEADERS
block on that reserved-by-peer promised stream. Without `END_STREAM`, the
promised stream enters the same tracked open-stream lifecycle used by
peer-created streams. With `END_STREAM`, it enters the same closed-by-peer
lifecycle. DATA on the reserved-by-peer promised stream before that response
HEADERS block keeps the existing `http2.protocol.invalid_frame_kind`
stream-state diagnostic boundary with reserved-by-peer active state,
`reserved_by_peer_requires_response_headers` rule provenance, and the bounded
frame-header preview used by other HTTP/2 protocol diagnostics.

Focused failures preserve the existing diagnostic families: associated stream
id zero and wrong-parity associated stream ids use
`http2.protocol.invalid_stream_id`; promised stream id zero and
client-initiated promised stream ids use `http2.protocol.invalid_stream_id`
with client receive rule provenance; payloads shorter than the promised-stream
field use `http2.protocol.invalid_payload_length`; unsupported promised
header blocks keep the HPACK fixture diagnostic shape.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  single-frame `PUSH_PROMISE` receive, emits the stripped promised header
  block, decodes it through the HPACK fixture path, and prints the
  reserved-by-peer stream state.
- The same checked case accepts a `PUSH_PROMISE` header block completed by a
  final CONTINUATION frame and verifies the same stripped HPACK fixture output
  and reserved-by-peer state.
- The same checked case accepts response HEADERS on the promised stream and
  verifies both the tracked open-stream and `END_STREAM` closed-by-peer
  lifecycle outcomes.
- Focused human and JSON examples under `../../../examples/specification/run/`
  check the rejected DATA-before-HEADERS transition through the existing
  invalid frame-kind diagnostic boundary.
- The same checked case covers associated stream id zero, wrong associated
  stream parity, promised stream id zero, wrong promised-stream parity, short
  payload, and unsupported HPACK fixture input through their focused
  diagnostic routes.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented receive boundary and route readers to the checked
  executable example.
