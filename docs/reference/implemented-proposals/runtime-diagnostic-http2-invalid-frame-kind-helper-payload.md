# Runtime Diagnostic HTTP/2 Invalid Frame-Kind Helper Payload

Status: implemented

This record preserves the completed HTTP/2 invalid frame-kind standard helper
runtime diagnostic payload slice from the runtime diagnostic payload proposal.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_protocol_invalid_frame_kind(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...)` carrying the byte offset,
actual frame kind, stream id, expected frame kind, active protocol state, rule
provenance, and frame-header preview chunk.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. The helper no longer
needs to register this diagnostic through the message-keyed backend side-table
bridge. The legacy bridge remains available for unrelated helpers that are
outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-invalid-frame-kind-helper-json/`
  checks that a direct connection-level helper call returns a rendered
  `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields.
- `../../../examples/specification/run/runtime-diagnostic-http2-stream-invalid-frame-kind-helper-json/`
  checks the same direct payload behavior for a nonzero stream id.
- `../../../examples/specification/run/http2-protocol-core-invalid-frame-kind-human/`,
  `../../../examples/specification/run/http2-protocol-core-invalid-frame-kind-json/`,
  `../../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-human/`,
  and
  `../../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-json/`
  keep the existing public human and JSON command protocol facts stable.
- `../../specification/run-json.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
