# Binary Schema Repeat Schema Payload Helpers

Status: implemented

This record preserves the completed nested schema payload slice for bounded
binary schema repeats from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept
`Repeat(count_field, SchemaName)` and
`Repeat(count_field, module::SchemaName)` when the count names an earlier
visible `Int` field and the payload schema is an eligible same-module binary
schema or a public imported binary schema resolved through a written `use`
path.

Decode exposes the repeated field as a list of the nested schema's decoded
record shape. Encode accepts that same list shape and writes each element
through the nested schema helper path. Decode truncation and encode
representation failures keep the outer repeated field path, append the
repeated element index, then append the nested schema field path.

## Evidence

- `../../../examples/specification/run/binary-schema-repeat-nested-decode/`
  checks same-module nested repeat decode.
- `../../../examples/specification/run/binary-schema-imported-repeat-nested-decode/`
  checks imported public nested repeat decode.
- `../../../examples/specification/run/binary-schema-repeat-nested-truncated-json/`
  checks same-module nested repeat truncation diagnostics.
- `../../../examples/specification/run/binary-schema-imported-repeat-nested-truncated-json/`
  checks imported nested repeat truncation diagnostics.
- `../../../examples/specification/run/binary-schema-repeat-nested-encode/`
  checks same-module nested repeat encode.
- `../../../examples/specification/run/binary-schema-imported-repeat-nested-encode/`
  checks imported public nested repeat encode.
- `../../../examples/specification/run/binary-schema-repeat-nested-encode-failure/`
  checks nested repeat encode field paths for representation failures.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
repeat count expression forms, byte-view repeat shapes, recursive repeated
nested schemas outside the existing helper eligibility checks, and mapping
behavior outside the implemented structural slices.
