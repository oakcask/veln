# HTTP/2 Request Header Validation

Status: implemented

This record closes the request-side header-list validation slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md`, `../../specification/run-json.md`, and
the checked examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol-core example decodes completed inbound HEADERS and final
CONTINUATION header blocks through the imported HPACK fixture module before
running request-header validation for fixture-marked request header lists.

The validation rejects request header lists that are missing `:method`,
`:scheme`, or `:path`, and rejects response-only `:status` on inbound
requests. Failures use the protocol-owned
`http2.protocol.invalid_request_header_list` diagnostic rather than schema or
HPACK fixture diagnostics.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks the
  integrated protocol-core path, including one accepted request fixture, a
  final CONTINUATION path missing `:method`, and a HEADERS path containing
  response-only `:status`.
- `../../../examples/specification/run/http2-protocol-core-request-headers-json/`
  checks the JSON projection for a missing required pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-request-headers-human/`
  checks the human projection for a response-only pseudo-header.

## Remaining Work

Full HPACK compression, complete RFC request-header validation, dynamic-table
policy, and socket integration remain outside this completed slice.
