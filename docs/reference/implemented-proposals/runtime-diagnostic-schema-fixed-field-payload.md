# Runtime Diagnostic Schema Fixed Field Payload

Status: implemented

This record preserves the completed generated binary schema fixed-field
mismatch payload slice from the runtime diagnostic payload proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Generated binary schema decode helpers now return a source-visible
`Err(RuntimeDiagnostic(..., RuntimeByteDiagnostic(...)))` payload when a
visible fixed exact-width field decodes to a value different from the schema's
fixed value. The byte detail carries the decoded-stream `ByteOffset`, the
schema-local field path, fixed-value facts, and the bounded byte preview.

Command output keeps the rendered `RuntimeDiagnostic(...)` as the result value
while projecting the same payload into the existing public
`schema.fixed_field_mismatch` human diagnostic and
`details.byte_diagnostic` JSON shape. Legacy side-table support remains for
unmigrated schema and helper diagnostics.

## Evidence

- `../../../examples/specification/run/binary-schema-fixed-field-mismatch-json/`
  checks that a generated fixed-field mismatch keeps the rendered
  `RuntimeDiagnostic(...)` result value, projects the existing byte diagnostic
  fields, and exposes the payload shape through result-value assertions.
- `../../../examples/specification/run/binary-schema-fixed-field-mismatch-human/`
  checks that human output keeps the focused `schema.fixed_field_mismatch`
  diagnostic and related byte notes.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the implemented behavior and
  route readers to executable evidence.
