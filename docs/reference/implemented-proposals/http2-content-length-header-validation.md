# HTTP/2 Content-Length Header Validation

Status: implemented

This record closes the checked `content-length` header-list validation slices
from `../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives
in `../../specification/execution.md`, `../../specification/run-json.md`, and
the checked examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol-core example decodes completed inbound HEADERS and final
CONTINUATION header blocks through the imported HPACK fixture module before
running request and response header-list validation. Request validation covers
both fixture-marked `content-length` list values and decoded `content-length`
header values carried beside the request pseudo-header marker. Response
validation covers fixture-marked list values and decoded `content-length`
header values carried beside a static response `:status` pseudo-header.

Request and response validation accepts header lists without `content-length`,
with one valid decimal `content-length` value, or with repeated
`content-length` values when every value is the same valid decimal spelling.
It rejects empty, non-decimal, signed, whitespace-padded, and
negative-looking values with failed fact `content_length_invalid`. It rejects
repeated valid decimal values that differ with failed fact
`content_length_mismatch`. Request failures project through
`http2.protocol.invalid_request_header_list`; response failures project
through `http2.protocol.invalid_response_header_list`.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  request and response `content-length` header lists with one value and with
  repeated matching values, plus rejected mismatch, empty, non-decimal, signed,
  whitespace-padded, and negative-looking values for both request and
  response lists. The same case checks production request `content-length`
  header values that are not part of the request marker, and source-visible
  response static-name `content-length` header values carried beside a static
  `:status` pseudo-header, on completed HEADERS and final CONTINUATION paths
  where applicable.
- `../../../examples/specification/run/http2-protocol-core-request-headers-content-length-json/`
  checks JSON projection for an invalid request `content-length` value.
- `../../../examples/specification/run/http2-protocol-core-request-headers-content-length-human/`
  checks human projection for mismatched request `content-length` values.
- `../../../examples/specification/run/http2-protocol-core-response-headers-content-length-json/`
  checks JSON projection for an invalid response `content-length` value.
- `../../../examples/specification/run/http2-protocol-core-response-headers-content-length-human/`
  checks human projection for mismatched response `content-length` values.

## Remaining Work

Full HPACK compression, trailer-field validation beyond the checked fixture
boundary, and socket integration remain outside this completed slice. The
completed body accounting follow-up is archived in
[HTTP/2 Content-Length Body Accounting](http2-content-length-body-accounting.md).
