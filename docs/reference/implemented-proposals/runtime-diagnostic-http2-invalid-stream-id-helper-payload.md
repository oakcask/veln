# Runtime Diagnostic HTTP/2 Invalid Stream Id Helper Payload

Status: implemented

This record preserves the completed HTTP/2 invalid stream id standard helper
runtime diagnostic payload slice from the runtime diagnostic payload proposal.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable case
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_protocol_invalid_stream_id(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...)` carrying the byte offset,
frame kind, stream id, required stream id domain, endpoint role, active
protocol state, rule provenance, and frame-header preview chunk.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. The helper no longer
needs to register this diagnostic through the message-keyed backend side-table
bridge. The legacy bridge remains available for unrelated helpers that are
outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-invalid-stream-id-helper-json/`
  checks that a direct helper call returns a rendered
  `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields.
- `../../../examples/specification/run/http2-protocol-core-invalid-stream-id-human/`,
  `../../../examples/specification/run/http2-protocol-core-invalid-stream-id-json/`,
  `../../../examples/specification/run/http2-protocol-core-invalid-stream-reference-human/`,
  and
  `../../../examples/specification/run/http2-protocol-core-invalid-stream-reference-json/`
  keep the existing public human and JSON command protocol facts stable.
- `../../specification/run-json.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
