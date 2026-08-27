---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Runtime Diagnostic HTTP/2 Continuation Helper Payload

This record preserves the completed HTTP/2 continuation-expected standard
helper runtime diagnostic payload slice from the runtime diagnostic payload
proposal. Current behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

`http2_protocol_continuation_expected(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...)` carrying the byte
offset, actual frame kind, actual stream id, expected stream id, started
frame kind, started byte offset, active continuation state, accumulated
header-block byte count, rule provenance, and inspected frame-header preview
chunk.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. Human output uses the
same focused continuation-ordering diagnostic and related notes as the
historical aggregate evidence. The helper no longer needs to register this
diagnostic through the message-keyed backend side-table bridge. The legacy
bridge remains available for unrelated helpers that are outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-continuation-helper-json/`
  checks that a direct call to
  `http2_protocol_continuation_expected(...)` returns a rendered
  `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields.
- `../../../examples/specification/run/http2-protocol-core-continuation-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-continuation-json/`
  keep the existing public human and JSON command output stable.
- `../../specification/run-json.md`,
  `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects.md` summarize the implemented
  behavior and route readers to executable evidence.
