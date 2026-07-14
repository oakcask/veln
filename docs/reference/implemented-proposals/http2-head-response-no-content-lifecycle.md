# HTTP/2 HEAD Response No-Content Lifecycle

Status: implemented

This record closes the client-side HEAD response body slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md` and the checked protocol-core examples
under `../../../examples/specification/run/`.

## Implemented Behavior

Accepted outbound request HEADERS retain whether their request method is
`HEAD`. When response HEADERS for that stream select the final response, the
stream enters the zero-content receive state regardless of final status. The
same transition applies after direct HEADERS, final CONTINUATION assembly, and
zero or more informational responses.

Response `content-length` remains accepted metadata for HEAD but does not
install an expected received body length. Direct response `END_STREAM`, empty
DATA termination, and padding-only DATA termination are accepted. Nonempty
DATA uses the existing `http2.protocol.content_length_mismatch` failure with
expected length zero, `head-response` active state, and
`rfc9110_head_response_body` provenance. Rejection preserves the pre-frame
stream, receive-window, HPACK, and output state.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks outbound
  method retention, direct and continued final responses, an informational
  response before the final response, response `content-length`, direct
  `END_STREAM`, empty and padding-only DATA termination, rejected nonempty
  DATA, rejection atomicity, and a GET regression.
- `../../../examples/specification/run/http2-protocol-core-head-response-data-human/`
  checks the focused human diagnostic, method-derived active state, byte
  preview, and rule provenance.

## Remaining Work

CONNECT tunnel response semantics, server-side outbound response-body
enforcement, socket integration, and unrelated response-status policy remain
outside this completed slice.
