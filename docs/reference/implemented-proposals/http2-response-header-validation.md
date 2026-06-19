# HTTP/2 Response Header Validation

Status: implemented

This record closes the response-side header-list validation slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md`, `../../specification/run-json.md`, and
the checked examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol-core example decodes completed inbound HEADERS and final
CONTINUATION header blocks through the imported HPACK fixture module before
running response-header validation for fixture-marked response header lists.

The validation rejects response header lists that omit `:status`, duplicate
`:status`, carry request-only `:method`, `:scheme`, or `:path`, or place
`:status` after a regular header. Failures use the protocol-owned
`http2.protocol.invalid_response_header_list` diagnostic rather than schema or
HPACK fixture diagnostics.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks the
  integrated protocol-core path, including one accepted response fixture, a
  final CONTINUATION path missing `:status`, duplicate `:status`, request-only
  `:method`, and `:status` after a regular `server` header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-json/`
  checks the JSON projection for a missing required pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-human/`
  checks the human projection for a request-only pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-duplicate-json/`
  checks the JSON projection for a duplicate pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-order-human/`
  checks the human projection for a pseudo-header after a regular header.

## Remaining Work

Full HPACK compression, complete RFC response-header validation,
dynamic-table policy, and socket integration remain outside this completed
slice.
