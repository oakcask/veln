# Runtime Diagnostic HTTP/2 Preface Payloads

Status: implemented

This record preserves the completed HTTP/2 client connection preface runtime
diagnostic payload slice from the runtime diagnostic payload proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

HTTP/2 client connection preface result failures can now be carried as
ordinary `Err(RuntimeDiagnostic(...))` values instead of depending only on
backend-local side-table registrations keyed by rendered error messages.
`RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...)` carries the byte offset,
pending byte count, expected preface length, active protocol state, rule
provenance, and bounded pending-byte preview. It projects the
`http2.protocol.partial_preface` id through the existing human diagnostic and
`details.protocol_diagnostic` JSON shapes.

`RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...)` carries the byte offset,
expected byte, actual byte, matched prefix count, expected preface length,
active protocol state, rule provenance, and bounded inspected-byte preview. It
projects the `http2.protocol.invalid_preface` id through the same command
surfaces.

This slice deliberately keeps legacy backend side-table support for existing
helpers while the remaining runtime diagnostic payload migration continues.

## Evidence

- `../../../examples/specification/run/http2-protocol-core-preface-partial-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-preface-partial-json/`
  check the source-visible partial preface projection and returned value
  shape.
- `../../../examples/specification/run/http2-protocol-core-preface-invalid-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-preface-invalid-json/`
  check the source-visible invalid preface projection and returned value
  shape.
- `../../specification/run-json.md`, `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
