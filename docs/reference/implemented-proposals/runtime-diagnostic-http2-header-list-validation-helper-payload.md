# Runtime Diagnostic HTTP/2 Header-List Validation Helper Payload

Status: implemented

This record preserves the completed HTTP/2 request and response header-list
validation standard helper runtime diagnostic payload slice from the runtime
diagnostic payload proposal. Current behavior is specified by
`../../specification/run-json.md`, `../../specification/commands.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_protocol_invalid_request_header_list(...)` and
`http2_protocol_invalid_response_header_list(...)` now return
`Result<(), RuntimeDiagnostic>`. On failure they return
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)` or
`RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...)` carrying the
byte offset, frame kind, stream id, failed header-list fact, offending header
name, decoded header names, active protocol state, and rule provenance.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. The helpers no longer
need to register these diagnostics through the message-keyed backend side-table
bridge. The legacy bridge remains available for unrelated helpers that are
outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-request-header-list-helper-json/`
  and
  `../../../examples/specification/run/runtime-diagnostic-http2-response-header-list-helper-json/`
  check that direct calls to the request and response helpers return rendered
  `RuntimeDiagnostic(...)` result values and structured
  `details.protocol_diagnostic` fields.
- `../../../examples/specification/run/runtime-diagnostic-http2-request-header-list-helper-human/`
  and
  `../../../examples/specification/run/runtime-diagnostic-http2-response-header-list-helper-human/`
  keep direct helper human output focused on the failed header-list fact while
  preserving related protocol context.
- `../../../examples/specification/run/http2-protocol-core-request-headers-json/`,
  `../../../examples/specification/run/http2-protocol-core-request-headers-human/`,
  `../../../examples/specification/run/http2-protocol-core-response-headers-json/`,
  and
  `../../../examples/specification/run/http2-protocol-core-response-headers-human/`
  keep the existing protocol-core request and response header-list public
  output stable.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
