# Runtime Diagnostic Encode Value Payload

Status: implemented

This record preserves the completed generated encode value-diagnostic payload
slice from the runtime diagnostic payload proposal. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable case under `../../../examples/specification/run/`.

## Completed Behavior

Generated binary schema encode failures can now be projected at a
source-visible reporting boundary as ordinary
`Err(RuntimeDiagnostic(..., RuntimeValueDiagnostic(...)))` values. The value
detail carries the schema-local field path and encode failure reason, and
command output projects those fields into the existing public
`details.value_diagnostic` JSON shape.

The implemented payload slice covers generated encode ids accepted by the
command-facing encode diagnostic projection, including
`codec.encode_value_unrepresentable`. Legacy `EncodeError(...)` and
`EncodeStep::Invalid(EncodeError(...))` result projections remain supported for
compatibility and for generated helpers that have not moved to a
source-visible payload boundary.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-payload-encode-value-json/`
  checks that a generated `codec.encode_value_unrepresentable` failure keeps
  the rendered `RuntimeDiagnostic(...)` result value while projecting
  `details.value_diagnostic` from `RuntimeValueDiagnostic(...)`.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the implemented behavior and
  route readers to executable evidence.
