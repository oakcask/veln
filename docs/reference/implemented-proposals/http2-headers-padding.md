# HTTP/2 HEADERS Padding

Status: implemented

This record preserves the completed inbound PADDED `HEADERS` slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

Inbound request headers and trailers read the pad-length octet before an
optional five-octet PRIORITY section. The receive path excludes that prefix
and the declared trailing padding from the HPACK fragment while preserving
END_HEADERS, END_STREAM, continuation assembly, priority state, stream
lifecycle, and header-list validation.

Missing prefix bytes and padding beyond the payload remaining after required
prefix fields use `http2.protocol.invalid_payload_length`. Rejection occurs
before HPACK, continuation, priority, stream, flow-control, settings,
shutdown, or peer-stream high-water state changes; ordinary input consumption
is retained.

## Evidence

- Historical aggregate evidence checks padded
  request headers, padded trailers, PADDED with PRIORITY, a padded fragment
  completed by CONTINUATION, absent prefix bytes, a truncated combined prefix,
  excessive padding, unpadded HPACK bytes, priority state, and rejection
  atomicity.
- `../../../examples/specification/run/http2-protocol-core-headers-padding-human/`
  fixes the human diagnostic, related frame fact, byte preview, active state,
  and rule provenance.
- `../../../examples/specification/run/http2-protocol-core-headers-padding-json/`
  fixes the matching structured diagnostic fields.
