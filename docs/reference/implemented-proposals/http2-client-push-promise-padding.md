# HTTP/2 Client PUSH_PROMISE Padding

Status: implemented

This record preserves the completed client-side inbound PADDED `PUSH_PROMISE`
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable case under
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

On an open associated client-created stream, a `PUSH_PROMISE` frame with the
PADDED flag reads its one-byte pad length before the four-byte promised stream
id. The payload must contain that five-byte prefix and at least the declared
number of trailing padding bytes.

After validation, the receive path removes the pad-length byte, promised
stream id, and trailing padding from the header-block fragment. Only that
unpadded fragment enters HPACK decode and promised request-header validation.
The same rule applies when a final CONTINUATION frame completes the block.
Zero padding follows the same accepted path.

Truncated padded prefixes and padding beyond the remaining payload use
`http2.protocol.invalid_payload_length`. The diagnostic keeps the failed
payload fact primary and records the frame kind, associated stream, observed
or available count, active state, rule provenance, and inspected payload
bytes through the existing structured projection. Validation occurs before
HPACK, continuation, promised-stream reservation, local-settings, shutdown,
receive-credit, or stream lifecycle state changes.

## Evidence

- The aggregate protocol-core case accepts a padded single-frame block and
  checks that only the unpadded bytes reach HPACK and promised-stream
  reservation.
- The same case accepts a padded block completed by CONTINUATION and verifies
  the unpadded accumulated block.
- Zero padding is accepted.
- Empty and four-byte padded payloads cover truncated prefixes.
- A pad length greater than the bytes after the promised stream id covers
  excessive padding.
- Every rejected case checks that promised-stream, HPACK, continuation,
  settings, shutdown, flow-control, and lifecycle state remains unchanged
  apart from ordinary input consumption.
- An excessive-padding case starts with an active continuation, disabled local
  push settings, and two differently configured inbound lifecycle entries. It
  proves padding failure takes precedence and preserves the complete carried
  state.
- Focused human and JSON cases under
  `../../../examples/specification/run/http2-protocol-core-push-promise-padding-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-push-promise-padding-json/`
  fix the command-facing count, frame, provenance, and byte-preview details.
