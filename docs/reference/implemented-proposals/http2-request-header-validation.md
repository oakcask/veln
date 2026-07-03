# HTTP/2 Request Header Validation

Status: implemented

This record closes the request-side header-list validation slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md`, `../../specification/run-json.md`, and
the checked examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol-core example decodes completed inbound HEADERS and final
CONTINUATION header blocks through the imported HPACK fixture module and the
source-visible static decoder before running request-header validation for
decoded request header lists.

The validation rejects request header lists that duplicate a request
pseudo-header, place a request pseudo-header after a regular header, omit
`:method`, `:scheme`, or `:path`, carry response-only `:status` on inbound
requests, carry uppercase ordinary header names, or carry ordinary header
names outside the HTTP field-name token shape. The follow-up connection-specific
header slice also rejects `connection`, `keep-alive`, `proxy-connection`,
`transfer-encoding`, and `upgrade` as ordinary request headers. The
request `:scheme` value slice accepts `http` and `https`, and rejects any
other fixture-marked value or source-visible HPACK static-name literal value
with failed fact `scheme_value_not_http_or_https`. The request `:method`
value slice rejects
fixture-marked empty values with failed fact `method_value_empty`, and the
request `:path` value slice rejects fixture-marked empty values with failed
fact `path_value_empty` after `:path` presence has been confirmed. The
request `:authority` value slice rejects fixture-marked invalid values and
checked source-visible HPACK static-name literal values with failed fact
`authority_value_invalid`. Failures use the protocol-owned
`http2.protocol.invalid_request_header_list` diagnostic rather than schema or
HPACK fixture diagnostics.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks the
  integrated protocol-core path, including one accepted request fixture, a
  request fixture with a lowercase ordinary `host` header, accepted
  `:scheme` values `http` and `https` through completed HEADERS and final
  CONTINUATION paths, an unsupported fixture-marked `:scheme` value,
  accepted source-visible HPACK static-name literal `:scheme` values, and
  rejected source-visible HPACK static-name literal `:scheme` values across
  literal-without-indexing, literal-with-indexing, literal-never-indexed, and
  final CONTINUATION paths, an empty `:method` value, an empty `:path` value
  after method and scheme presence are satisfied, an invalid fixture-marked
  `:authority` value, accepted source-visible HPACK static-name literal
  `:authority` values, and rejected source-visible HPACK static-name literal
  `:authority` values across literal-without-indexing, literal-with-indexing,
  literal-never-indexed, and final CONTINUATION paths, a final CONTINUATION
  path missing `:method`, a HEADERS path containing response-only `:status`, a
  duplicate `:method`, and a `:method` after a regular `host` header, plus
  uppercase and token-invalid ordinary request header names and the checked
  connection-specific ordinary request header names.
- `../../../examples/specification/run/http2-protocol-core-request-headers-json/`
  checks the JSON projection for a missing required pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-request-headers-human/`
  checks the human projection for a response-only pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-request-headers-duplicate-json/`
  checks the JSON projection for a duplicate pseudo-header.
- `../../../examples/specification/run/http2-protocol-core-request-headers-order-human/`
  checks the human projection for a pseudo-header after a regular header.
- `../../../examples/specification/run/http2-protocol-core-request-headers-uppercase-json/`
  checks the JSON projection for an uppercase ordinary header name.
- `../../../examples/specification/run/http2-protocol-core-request-headers-token-human/`
  checks the human projection for an ordinary header name outside the HTTP
  field-name token shape.
- `../../../examples/specification/run/http2-protocol-core-request-headers-connection-specific-json/`
  checks the JSON projection for a connection-specific ordinary request
  header name.
- `../../../examples/specification/run/http2-protocol-core-request-headers-scheme-json/`
  checks the JSON projection for an unsupported request `:scheme` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-scheme-human/`
  checks the human projection for an unsupported request `:scheme` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-path-empty-json/`
  checks the JSON projection for an empty request `:path` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-path-empty-human/`
  checks the human projection for an empty request `:path` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-method-empty-json/`
  checks the JSON projection for an empty request `:method` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-method-empty-human/`
  checks the human projection for an empty request `:method` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-authority-invalid-json/`
  checks the JSON projection for an invalid request `:authority` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-authority-invalid-human/`
  checks the human projection for an invalid request `:authority` value.

## Remaining Work

Full HPACK compression, response-header production validation, dynamic-table
policy, and socket integration remain outside this completed slice.
