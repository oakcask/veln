# Runtime Diagnostic HTTP/2 WINDOW_UPDATE Payload

Status: implemented

This record preserves the completed HTTP/2 invalid `WINDOW_UPDATE` increment
runtime diagnostic payload slice from the runtime diagnostic payload proposal.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

HTTP/2 `WINDOW_UPDATE` frames with an invalid increment can now be carried as
ordinary `Err(RuntimeDiagnostic(...))` values instead of depending only on
backend-local side-table registration keyed by the rendered error message.
`RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(...)` carries the
byte offset, stream id, observed increment, accepted range, active protocol
state, rule provenance, and bounded payload byte preview. Command projection
keeps the same `http2.protocol.invalid_window_update_increment` human
diagnostic and `details.protocol_diagnostic` JSON shape while `details.value`
preserves the rendered source-visible payload.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-window-update-increment-human/`
  checks the source-visible human projection for a zero connection-level
  `WINDOW_UPDATE` increment.
- `../../../examples/specification/run/http2-protocol-core-window-update-increment-json/`
  checks the source-visible JSON projection, returned value shape, accepted
  range facts, active state, rule provenance, and bounded payload byte
  preview.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
