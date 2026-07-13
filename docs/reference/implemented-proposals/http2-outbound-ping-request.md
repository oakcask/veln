# HTTP/2 Outbound PING Request

Status: implemented

This record preserves the completed local outbound PING request slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The pure protocol core accepts a local PING request intent only when its opaque
payload contains exactly eight bytes. An accepted intent returns exactly one
immutable output chunk containing the nine-byte HTTP/2 frame header followed
by the unchanged payload. The header carries length `8`, kind `6`, flags `0`,
and stream id `0`.

Short and long payloads return an `OutboundPingRejected` decision containing
the observed payload length and the focused
`http2.protocol.invalid_payload_length` failure. The length check happens
before frame encoding, so rejected intents return no output chunks.

The slice does not add outstanding-request tracking, ACK correlation,
deadlines, round-trip measurement, retries, keepalive policy, sockets, or
transport adapter changes. Existing inbound PING validation and automatic ACK
behavior remain unchanged.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks the exact
  accepted frame bytes and one immutable output chunk.
- The same checked case covers both a seven-byte and a nine-byte payload,
  asserts the typed payload-length rejection reason and observed length, and
  pins each rejected intent to an empty output chunk list.
- `../../specification/execution.md` summarizes the current behavior and
  routes readers to the executable evidence.
