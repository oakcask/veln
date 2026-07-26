# HTTP/2 Core PING Transitions

Status: implemented

This record preserves the completed bounded PING request and response slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/http2.md`, `../../specification/execution.md`, the
adjacent standard-library tests, and the checked executable case
`../../../examples/specification/run/http2-core-ping-transitions/`.

## Completed Behavior

The pure protocol core accepts a local PING request intent only when its opaque
payload contains exactly eight bytes. An accepted intent returns exactly one
immutable output chunk containing the nine-byte HTTP/2 frame header followed
by the unchanged payload. The header carries length `8`, kind `6`, flags `0`,
and stream id `0`.

Short and long payloads return a public rejected decision containing the
observed payload length and the shared focused
`http2.protocol.invalid_payload_length` failure from the core payload-length
validator. The length check happens before frame encoding, so rejected intents
return no output chunks.

For an already validated received non-ACK PING, the pure protocol core returns
one ACK response chunk with the unchanged eight-octet payload. The ACK header
carries length `8`, kind `6`, flags `1`, and stream id `0`. A received PING
ACK returns an explicit no-response decision with no output chunk so the
transition cannot create an ACK loop.

The slice does not add outstanding-request tracking, ACK correlation,
deadlines, round-trip measurement, retries, keepalive policy, sockets, or
transport adapter changes.

## Evidence

- `../../../crates/veln-stdlib/veln/http2/core_test.veln` checks the exact
  accepted frame bytes, seven- and nine-byte rejections, typed payload-length
  context, empty failure output, immutable input preservation, exact ACK bytes,
  payload preservation, and received-ACK no-response behavior.
- `../../../examples/specification/run/http2-core-ping-transitions/` imports
  `http2::core` from `std` and records the accepted request, representative
  failure, ACK response, no-response decision, and emitted bytes.
- `../../../examples/specification/run/http2-protocol-core/` retains wider
  integration and complete-stdout coverage while calling the public facade.
