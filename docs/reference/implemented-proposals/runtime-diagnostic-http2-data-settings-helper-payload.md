# Runtime Diagnostic HTTP/2 DATA And SETTINGS Helper Payload

Status: implemented

This record preserves the completed HTTP/2 standard helper runtime diagnostic
payload slice for invalid DATA padding and unexpected SETTINGS ACK failures.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

These standard helpers now return `Result<(), RuntimeDiagnostic>` directly:

- `http2_protocol_invalid_data_padding(...)`
- `http2_protocol_unexpected_settings_ack(...)`

On failure each helper returns `Err(RuntimeDiagnostic(...))` with the matching
HTTP/2 protocol detail constructor. Command recording projects
`details.protocol_diagnostic` from the returned value while preserving the
rendered `RuntimeDiagnostic(...)` in `details.value`. The legacy side-table
bridge remains available for unrelated helpers outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-data-padding-helper-json/`
  checks the direct invalid DATA padding helper payload.
- `../../../examples/specification/run/runtime-diagnostic-http2-settings-ack-helper-json/`
  checks the direct unexpected SETTINGS ACK helper payload.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
