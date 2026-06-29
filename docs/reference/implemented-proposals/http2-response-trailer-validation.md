# HTTP/2 Response Trailer Validation

Status: implemented

This record closes the inbound response-trailer validation slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md` and the checked HTTP/2 protocol-core
example under `../../../examples/specification/run/http2-protocol-core/`.

## Implemented Behavior

The HTTP/2 protocol-core example records when an inbound response HEADERS
sequence opens a stream. A later HEADERS sequence on that open response stream
is treated as response trailers only when it carries peer `END_STREAM`.

Accepted ordinary response trailer fields close the stream by peer without
consuming connection or stream receive-window credit. The behavior is checked
for both completed HEADERS and final CONTINUATION paths.

A second response HEADERS block without peer `END_STREAM` is rejected in
response-trailer state. Response trailer validation rejects pseudo-headers,
uppercase ordinary names, ordinary names outside the HTTP field-name token
shape, connection-specific ordinary names, and invalid `te` values through
`http2.protocol.invalid_response_header_list` with active state
`response-trailers`.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  response trailers, a post-trailer DATA rejection, a missing peer
  `END_STREAM` rejection with response-trailer state, final CONTINUATION
  completion, and response header-list diagnostics for pseudo-header,
  uppercase, token-invalid, connection-specific, and invalid `te` trailer
  fields.
- A focused response-trailer JSON diagnostic projection checks active state
  `response-trailers`.
- A focused response-trailer human diagnostic projection checks
  `http2.protocol.invalid_response_header_list`, response-trailer primary
  text, decoded trailer names, bounded byte preview, active
  `response-trailers` state, and rule provenance.

## Remaining Work

Full HPACK compression, broader HTTP/2 protocol-core behavior, and socket
integration remain outside this completed slice.
