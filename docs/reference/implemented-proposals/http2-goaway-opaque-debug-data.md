# HTTP/2 GOAWAY Opaque Debug Data

Status: implemented

This record preserves the completed GOAWAY opaque debug-data slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
historical aggregate evidence.

## Completed Behavior

Inbound GOAWAY accepts payloads of at least eight bytes. The first eight bytes
remain the schema-declared last-stream-id and error-code fields, while every
trailing byte remains an immutable opaque byte sequence in the ordinary frame
receive result. The protocol core does not interpret or validate that sequence
as text. A shorter payload retains the focused
`http2.protocol.invalid_payload_length` failure and its existing diagnostic
precedence.

Outbound GOAWAY accepts an immutable opaque debug-data chunk, appends it
unchanged after the encoded fixed fields, and derives the frame payload length
from the complete encoded payload. Empty debug data emits the original
eight-byte payload. Last-stream-id representation checks, repeated-GOAWAY
narrowing, and graceful-drain transitions remain unchanged.

## Evidence

- Historical aggregate evidence checks empty and
  non-empty inbound debug data and compares every preserved trailing byte.
- The same case receives non-text debug bytes while an admitted stream remains
  active and verifies the ordinary graceful-shutdown transition.
- The outbound checked chunk contains the exact fixed fields followed by the
  supplied non-text bytes, with the complete payload length in the frame
  header.
- The existing seven-byte case retains the protocol-owned payload-length
  diagnostic with observed length `7` and required length `8`.
