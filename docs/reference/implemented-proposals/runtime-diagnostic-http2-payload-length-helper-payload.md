# Runtime Diagnostic HTTP/2 Payload-Length Helper Payload

Status: implemented

This record preserves the completed HTTP/2 invalid payload-length standard
helper runtime diagnostic payload slice from the runtime diagnostic payload
proposal. Current behavior is specified by `../../specification/run-json.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable case
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_protocol_invalid_payload_length(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...)` carrying the byte
offset, frame kind, stream id, observed payload length, expected payload
length, active protocol state, rule provenance, and bounded payload byte
preview.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. The helper no longer
needs to register this diagnostic through the message-keyed backend side-table
bridge. The legacy bridge remains available for unrelated helpers that are
outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-window-update-payload-length-helper-json/`
  checks that a direct call to
  `http2_protocol_invalid_payload_length(...)` returns a rendered
  `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields for the `WINDOW_UPDATE` fixed payload
  length case.
- `../../../examples/specification/run/http2-protocol-core-settings-ack-length-human/`,
  `../../../examples/specification/run/http2-protocol-core-settings-ack-length-json/`,
  `../../../examples/specification/run/http2-protocol-core-ping-length-human/`,
  `../../../examples/specification/run/http2-protocol-core-ping-length-json/`,
  `../../../examples/specification/run/http2-protocol-core-goaway-length-human/`,
  `../../../examples/specification/run/http2-protocol-core-goaway-length-json/`,
  `../../../examples/specification/run/http2-protocol-core-rst-stream-length-human/`,
  and
  `../../../examples/specification/run/http2-protocol-core-rst-stream-length-json/`
  keep the existing public human and JSON command output stable.
- `../../specification/run-json.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
