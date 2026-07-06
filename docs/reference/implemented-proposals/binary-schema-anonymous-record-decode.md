# Binary Schema Anonymous Record Decode

Status: implemented

This record preserves the completed generated decode helper slice for binary
schema fields whose type is an anonymous record made only from implemented
exact-width unsigned primitive leaves. Current behavior is specified by
`../../specification/source-surface.md`, `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode helpers accept a visible field whose schema type
is an anonymous record. Each nested field must be an implemented exact-width
unsigned primitive leaf. Decode reads the nested leaves in source order and
exposes the same anonymous record shape at the outer schema field.

Runtime truncation inside the anonymous record uses the existing
`schema.truncated_field` byte diagnostic shape. The field path keeps the outer
schema field segment and appends the nested anonymous record field segment,
without inserting a synthetic nested schema segment.

Encode helpers for anonymous record fields remain outside this binary-schema
slice.

## Evidence

- `../../../examples/specification/run/binary-schema-anonymous-record-decode/`
  checks successful decode of a visible anonymous record field made from
  implemented exact-width unsigned primitive leaves.
- `../../../examples/specification/run/binary-schema-anonymous-record-truncated-json/`
  checks nested truncation JSON with the preserved outer field path and nested
  anonymous record field path.

## Remaining Work

The broader schema declaration surface proposal remains open for generated
runtime helper bindings outside the implemented binary helper boundaries and
format-neutral helper boundaries. Anonymous record encode support in
`format binary` schemas remains outside this completed decode-only slice.
