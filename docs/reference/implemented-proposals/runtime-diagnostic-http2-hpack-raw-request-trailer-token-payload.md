# Runtime Diagnostic HTTP/2 HPACK Raw Request-Trailer Token Payload

Status: implemented

This record preserves the completed HTTP/2 HPACK raw request-trailer
invalid-token runtime diagnostic payload slice from the runtime diagnostic
payload proposal. Current behavior is specified by
`../../specification/run-json.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

An inbound request-trailer header-list failure caused by an ordinary field name
outside the HTTP field-name token shape decoded from raw HPACK data can now be
carried as an ordinary `Err(RuntimeDiagnostic(...))` value instead of depending
only on backend-local side-table registration keyed by the rendered error
message. `RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)` carries
the byte offset, frame kind, stream id, failed header fact, header name,
decoded header names, active protocol state, and rule provenance. Command
projection keeps the same `http2.protocol.invalid_request_header_list` human
diagnostic and JSON `details.protocol_diagnostic` shape while `details.value`
preserves the rendered source-visible payload.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-hpack-raw-name-token-human/`
  checks the stable human diagnostic output for the source-visible payload.
- `../../../examples/specification/run/http2-protocol-core-hpack-raw-name-token-json/`
  checks the source-visible JSON projection, returned value shape, and existing
  protocol diagnostic fields for the raw HPACK request-trailer invalid-token
  name case.
- `../../specification/run-json.md` and `../../specification/execution.md`
  summarize the implemented behavior and route readers to executable evidence.
