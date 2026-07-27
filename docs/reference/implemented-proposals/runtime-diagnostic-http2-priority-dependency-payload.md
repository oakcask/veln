# Runtime Diagnostic HTTP/2 PRIORITY Dependency Payload

Status: implemented

This record preserves the completed HTTP/2 PRIORITY self-dependency runtime
diagnostic payload slice from the runtime diagnostic payload proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

HTTP/2 PRIORITY frames whose dependency stream id equals their own stream id
can now be carried as ordinary `Err(RuntimeDiagnostic(...))` values instead of
depending only on backend-local side-table registration keyed by the rendered
error message.
`RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...)` carries the byte
offset, stream id, dependency stream id, active protocol state, rule
provenance, and bounded PRIORITY payload byte preview. Command projection keeps
the same `http2.protocol.invalid_priority_dependency` human diagnostic and
`details.protocol_diagnostic` JSON shape while `details.value` preserves the
rendered source-visible payload.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-priority-dependency-human/`
  checks the source-visible human diagnostic projection.
- `../../../examples/specification/run/http2-protocol-core-priority-dependency-json/`
  checks the source-visible JSON projection, returned value shape, and existing
  protocol diagnostic fields.
- Historical aggregate evidence uses the
  source-visible payload route for integrated PRIORITY self-dependency
  projection coverage.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
