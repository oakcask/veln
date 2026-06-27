# Runtime Diagnostic HTTP/2 Invalid Stream Id Payload

Status: implemented

This record preserves the completed HTTP/2 invalid stream id runtime
diagnostic payload slice from the runtime diagnostic payload proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

HTTP/2 stream id domain failures can now be carried as ordinary
`Err(RuntimeDiagnostic(...))` values instead of depending only on
backend-local side-table registration keyed by the rendered error message.
`RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...)` carries the byte offset,
frame kind, stream id, required stream id domain, endpoint role, active
protocol state, rule provenance, and bounded frame-header byte preview.
Command projection keeps the same `http2.protocol.invalid_stream_id` human
diagnostic and `details.protocol_diagnostic` JSON shape while `details.value`
preserves the rendered source-visible payload.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-invalid-stream-id-human/`
  checks the source-visible human projection for a stream frame on the
  connection stream.
- `../../../examples/specification/run/http2-protocol-core-invalid-stream-id-json/`
  checks the source-visible JSON projection, returned value shape, and existing
  protocol diagnostic fields for an even client stream id.
- `../../../examples/specification/run/http2-protocol-core-invalid-stream-reference-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-invalid-stream-reference-json/`
  check connection-only frame stream references through the same payload route.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
