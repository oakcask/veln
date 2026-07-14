# HTTP/2 Informational Response Headers

Status: implemented

This record closes the inbound client-side informational response HEADERS
slice from `../../proposals/http2-sans-io-protocol-core.md`. Current behavior
lives in `../../specification/execution.md` and the checked protocol-core
example under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol core classifies a valid three-digit `1xx` response status
other than `101` as informational. An informational response retains the
stream's expectation of final response HEADERS, so another informational
response or the final response follows the response-header path rather than
the trailer path. The same transition applies after a complete HEADERS frame,
after final CONTINUATION assembly, and on a peer-promised stream.

Status `101` is rejected because HTTP/2 does not use the HTTP/1.1 switching
protocols mechanism. Informational HEADERS carrying `END_STREAM` are also
rejected. Both failures use the existing
`http2.protocol.invalid_response_header_list` boundary with a focused failed
response fact and existing response-header state and rule provenance.

DATA is rejected while the stream is waiting for final response HEADERS. Once
the final non-informational response is accepted, the existing DATA,
content-length, trailer, reset, and end-of-stream behavior remains in effect.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks one and
  multiple informational responses, final response transition, final
  CONTINUATION completion, a peer-promised stream, rejected status `101`,
  rejected informational `END_STREAM`, rejected DATA before the final
  response, and accepted DATA after the final response.
- `../../../examples/specification/run/http2-protocol-core-informational-end-stream-human/`
  checks the focused human diagnostic and its response state and rule notes.

## Remaining Work

Outbound server send-intents for informational responses, socket integration,
and unrelated HTTP/2 lifecycle extensions remain outside this completed
slice.
