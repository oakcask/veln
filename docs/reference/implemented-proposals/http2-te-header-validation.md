# HTTP/2 TE Header Validation

Status: implemented

This record closes the source-visible `te` ordinary-header value slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md`, `../../specification/run-json.md`, and
the checked examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol-core example decodes completed inbound HEADERS and final
CONTINUATION header blocks through the imported HPACK fixture module before
running request and response header-list validation for fixture-marked header
lists.

The validation accepts ordinary `te: trailers` on inbound request and
response header lists. A fixture-marked `te` header with any other value is
rejected through `http2.protocol.invalid_request_header_list` for requests
and `http2.protocol.invalid_response_header_list` for responses. The failed
fact is `te_header_value_not_trailers`, the reported header name is `te`, and
the rule provenance is `rfc9113_te_trailers_only`.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  `te: trailers` request validation through completed HEADERS, accepted
  `te: trailers` response validation through final CONTINUATION, and invalid
  request and response `te` values through the integrated protocol-core
  header-list boundary.
- `../../../examples/specification/run/http2-protocol-core-request-headers-te-json/`
  checks the JSON projection for an invalid request `te` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-te-human/`
  checks the human projection for an invalid request `te` value.
- `../../../examples/specification/run/http2-protocol-core-response-headers-te-json/`
  checks the JSON projection for an invalid response `te` value.
- `../../../examples/specification/run/http2-protocol-core-response-headers-te-human/`
  checks the human projection for an invalid response `te` value.

## Remaining Work

Full HPACK compression, general production header parsing, dynamic-table
policy, and socket integration remain outside this completed slice.
