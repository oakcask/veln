# Runtime Diagnostic HTTP/2 Invalid Frame-Kind Stream-State Payload

Status: implemented

This record preserves the completed HTTP/2 invalid frame-kind stream-state
runtime diagnostic payload slice from the runtime diagnostic payload proposal.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

A closed-by-peer stream-state frame-kind failure can now be carried as an
ordinary `Err(RuntimeDiagnostic(...))` value instead of depending only on
backend-local side-table registration keyed by the rendered error message.
`RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...)` carries the byte offset,
actual frame kind, stream id, expected frame kind, active protocol state, rule
provenance, and bounded frame-header byte preview. Command projection keeps
the same `http2.protocol.invalid_frame_kind` human diagnostic and
`details.protocol_diagnostic` JSON shape while `details.value` preserves the
rendered source-visible payload.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-stream-state-invalid-frame-kind-json/`
  checks the source-visible JSON projection, returned value shape, and existing
  protocol diagnostic fields for the closed-by-peer stream-state case.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
