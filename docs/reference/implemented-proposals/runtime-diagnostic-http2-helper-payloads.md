# Runtime Diagnostic HTTP/2 Helper Payloads

Status: implemented

This record preserves the completed HTTP/2 standard helper runtime diagnostic
payload slice from the runtime diagnostic payload proposal. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

These standard helpers now return `Result<(), RuntimeDiagnostic>` directly:

- `http2_protocol_invalid_window_update_increment(...)`
- `http2_protocol_content_length_mismatch(...)`
- `http2_protocol_invalid_priority_dependency(...)`
- `http2_protocol_stream_after_goaway(...)`
- `http2_peer_limit_flow_control_window_exceeded(...)`

On failure each helper returns `Err(RuntimeDiagnostic(...))` with the matching
HTTP/2 protocol detail constructor. Command recording projects
`details.protocol_diagnostic` from the returned value while preserving the
rendered `RuntimeDiagnostic(...)` in `details.value`. The legacy side-table
bridge remains available for unrelated helpers outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-window-update-increment-helper-json/`
  checks the direct invalid `WINDOW_UPDATE` increment helper payload.
- `../../../examples/specification/run/runtime-diagnostic-http2-content-length-helper-json/`
  checks the direct content-length mismatch helper payload.
- `../../../examples/specification/run/runtime-diagnostic-http2-priority-dependency-helper-json/`
  checks the direct invalid PRIORITY dependency helper payload.
- `../../../examples/specification/run/runtime-diagnostic-http2-stream-after-goaway-helper-json/`
  checks the direct stream-after-GOAWAY helper payload.
- `../../../examples/specification/run/runtime-diagnostic-http2-flow-control-helper-json/`
  checks the direct flow-control window helper payload.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
