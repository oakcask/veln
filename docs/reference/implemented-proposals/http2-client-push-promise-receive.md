# HTTP/2 Client PUSH_PROMISE Receive

Status: implemented

This record preserves the completed client-side peer-sent `PUSH_PROMISE`
receive slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

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
Later DATA or HEADERS behavior on the promised stream remains outside this
slice and uses the existing focused stream-state rejection boundary when
exercised.

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
- The same checked case covers associated stream id zero, wrong associated
  stream parity, promised stream id zero, wrong promised-stream parity, short
  payload, and unsupported HPACK fixture input through their focused
  diagnostic routes.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented receive boundary and route readers to the checked
  executable example.
