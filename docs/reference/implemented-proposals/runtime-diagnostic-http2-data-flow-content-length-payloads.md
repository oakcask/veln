# Runtime Diagnostic HTTP/2 Data Flow Content-Length Payloads

Status: implemented

This record preserves the completed HTTP/2 DATA padding, flow-control window,
and content-length mismatch runtime diagnostic payload slice from the runtime
diagnostic payload proposal. Current behavior is specified by
`../../specification/run-json.md`, `../../specification/commands.md`,
`../../specification/execution.md`, `../../specification/test-json.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

The migrated HTTP/2 projection boundaries now return ordinary
`Err(RuntimeDiagnostic(...))` values for:

- `http2.protocol.invalid_data_padding` through
  `RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...)`
- `http2.peer_limit.flow_control_window_exceeded` through
  `RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...)`
- `http2.protocol.content_length_mismatch` through
  `RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...)`

Command projection keeps the rendered `RuntimeDiagnostic(...)` as
`details.value` and projects the contained detail through the existing human
diagnostic and `details.protocol_diagnostic` JSON shapes. The legacy helper
functions remain available as compatibility shims while the remaining runtime
diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-data-padding-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-data-padding-json/`
  check source-visible DATA padding projection.
- `../../../examples/specification/run/http2-protocol-core-flow-control-human/`,
  `../../../examples/specification/run/http2-protocol-core-flow-control-json/`,
  `../../../examples/specification/run/http2-protocol-core-flow-control-connection-human/`,
  and
  `../../../examples/specification/run/http2-protocol-core-flow-control-connection-json/`
  check source-visible stream and connection flow-control projection.
- `../../../examples/specification/run/http2-protocol-core-content-length-early-human/`,
  `../../../examples/specification/run/http2-protocol-core-content-length-early-json/`,
  `../../../examples/specification/run/http2-protocol-core-content-length-over-human/`,
  and
  `../../../examples/specification/run/http2-protocol-core-content-length-over-json/`
  check source-visible content-length mismatch projection.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, `../../specification/test-json.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
