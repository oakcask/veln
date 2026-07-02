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
`:status`, carry request-only `:authority`, `:method`, `:scheme`, or `:path`,
place `:status` after a regular header, carry uppercase ordinary header
names, carry ordinary header names outside the HTTP field-name token shape,
or carry a `:status` value that is not exactly three ASCII decimal digits.
Failures use the protocol-owned
`http2.protocol.invalid_response_header_list` diagnostic rather than schema
or HPACK fixture diagnostics.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks the
  integrated protocol-core path, including accepted response fixtures with
  and without an ordinary `server` header, the accepted ordinary-header
  fixture through a final CONTINUATION path, a final CONTINUATION path missing
  `:status`, duplicate `:status`, request-only `:method` and `:authority`,
  `:status` after a regular `server` header, plus uppercase and token-invalid
  ordinary response header names. It also checks empty, short, long, and
  non-decimal `:status` values through fixture-marked response header lists
  and source-visible HPACK static-name literal values, with completed HEADERS
  and final CONTINUATION coverage.
- `../../../examples/specification/run/http2-protocol-core-response-headers-json/`
  checks the JSON projection for a missing required pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-human/`
  checks the human projection for a request-only pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-duplicate-json/`
  checks the JSON projection for a duplicate pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-order-human/`
  checks the human projection for a pseudo-header after a regular header.
- `../../../examples/specification/run/http2-protocol-core-response-headers-uppercase-json/`
  checks the JSON projection for an uppercase ordinary header name.
- `../../../examples/specification/run/http2-protocol-core-response-headers-token-human/`
  checks the human projection for an ordinary header name outside the HTTP
  field-name token shape.

## Remaining Work

Full HPACK compression, response-header rules beyond the checked pseudo-header
value and ordinary header-name shape, dynamic-table policy, and socket
integration remain outside this completed slice.
