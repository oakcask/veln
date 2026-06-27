# Runtime Diagnostic HTTP/2 SETTINGS Value Helper Payload

Status: implemented

This record preserves the completed HTTP/2 SETTINGS value peer-limit standard
helper runtime diagnostic payload slice from the runtime diagnostic payload
proposal. Current behavior is specified by `../../specification/run-json.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable case
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_peer_limit_settings_value_out_of_range(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2PeerLimitSettingsValueDiagnostic(...)` carrying the byte offset,
setting identifier, setting name, observed value, accepted range,
peer-limit provenance, and bounded SETTINGS item preview.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. The helper no longer
needs to register this diagnostic through the message-keyed backend side-table
bridge. The legacy bridge remains available for unrelated helpers that are
outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-settings-value-helper-json/`
  checks that a direct call to
  `http2_peer_limit_settings_value_out_of_range(...)` returns a rendered
  `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields.
- `../../../examples/specification/run/http2-protocol-core-settings-value-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-settings-value-json/`
  keep the existing public human and JSON command output stable.
- `../../specification/run-json.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
