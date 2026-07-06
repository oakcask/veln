# Runtime Diagnostic HTTP/2 Closed Helper Payload

Status: implemented

This record preserves the completed HTTP/2 closed-input standard helper
runtime diagnostic payload slice from the runtime diagnostic payload proposal.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable case
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_protocol_closed_with_pending(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...)` carrying the byte
offset, pending byte count, active continuation state, pending stream id,
starting frame kind, starting byte offset, accumulated header-block byte
count, rule provenance, and retained pending bytes.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. The helper no longer
needs to register this diagnostic through the message-keyed backend side-table
bridge. The legacy bridge remains available for unrelated helpers that are
outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-closed-helper-json/`
  checks that a direct call to `http2_protocol_closed_with_pending(...)`
  returns a rendered `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields.
- `../../specification/run-json.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
