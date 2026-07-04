# Binary Schema Dispatch Nested Repeat Helpers

Status: implemented

This record preserves the completed generated helper binding slice for binary
dispatch payload schemas whose nested payload field contains a bounded repeat
over an eligible nested binary schema. Current behavior is specified by
`../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode helpers accept a closed dispatch payload schema
whose selected nested payload decodes a `Repeat(count_field, NestedSchema)`
field after the count field has decoded as `Int`. The repeated payload exposes
the nested schema-local visible record shape as `List<NestedRecord>` inside the
dispatch payload record.

Runtime failures from the repeated nested payload keep the parent dispatch
field path, append the selected payload schema, append the repeated field and
element index, and then append the nested schema field path.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-nested-repeat-decode/`
  checks successful dispatch payload decode with a bounded repeated nested
  schema field.
- `../../../examples/specification/run/binary-schema-dispatch-nested-repeat-truncated-json/`
  checks truncation diagnostics with the parent dispatch field path, repeated
  element index, and nested schema field path.

## Remaining Work

The broader schema declaration surface proposal remains open for generated
runtime decode bindings outside the implemented binary helper boundaries and
format-neutral recursive visible-shape helper boundary.
