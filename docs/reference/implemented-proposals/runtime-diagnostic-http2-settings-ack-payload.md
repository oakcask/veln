# Runtime Diagnostic HTTP/2 SETTINGS ACK Payload

Status: implemented

This record preserves the completed HTTP/2 unexpected SETTINGS ACK runtime
diagnostic payload slice from the runtime diagnostic payload proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

An HTTP/2 SETTINGS ACK received while no local SETTINGS batch is outstanding
can now be carried as an ordinary `Err(RuntimeDiagnostic(...))` value instead
of depending only on backend-local side-table registration keyed by the
rendered error message.
`RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...)` carries the byte
offset, active protocol state, rule provenance, and bounded frame-header byte
preview. Command projection supplies the fixed SETTINGS ACK frame facts and
projects the `http2.protocol.unexpected_settings_ack` id through the existing
human diagnostic and `details.protocol_diagnostic` JSON shapes.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-settings-unexpected-ack-human/`
  checks the source-visible human diagnostic projection.
- `../../../examples/specification/run/http2-protocol-core-settings-unexpected-ack-json/`
  checks the source-visible JSON projection and returned value shape.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
