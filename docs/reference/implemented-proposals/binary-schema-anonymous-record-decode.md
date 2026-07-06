# Binary Schema Anonymous Record Decode

Status: implemented

This record preserves the completed generated decode helper slice for binary
schema fields whose type is an anonymous record made from implemented
exact-width unsigned primitive leaves, with one optional nested anonymous
record field made from the same leaves. Current behavior is specified by
`../../specification/source-surface.md`, `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode helpers accept a visible field whose schema type
is an anonymous record. Each nested field must be an implemented exact-width
unsigned primitive leaf, except that the outer anonymous record may contain one
nested anonymous record field whose fields are implemented exact-width
unsigned primitive leaves. Decode reads the leaves in source order and exposes
the same anonymous record shape at the outer schema field.

Runtime truncation inside the anonymous record uses the existing
`schema.truncated_field` byte diagnostic shape. The field path keeps the outer
schema field segment and appends each anonymous record field segment down to
the failed primitive, without inserting a synthetic nested schema segment.

Encode helpers for anonymous record fields remain outside this binary-schema
slice.

## Evidence

- `../../../examples/specification/run/binary-schema-anonymous-record-decode/`
  checks successful decode of a visible anonymous record field made from
  implemented exact-width unsigned primitive leaves.
- `../../../examples/specification/run/binary-schema-anonymous-record-truncated-json/`
  checks nested truncation JSON with the preserved outer field path and nested
  anonymous record field path.
- `../../../examples/specification/run/binary-schema-nested-anonymous-record-decode/`
  checks successful decode through the compatibility helper and explicit
  schema decode expression for one nested anonymous record field.
- `../../../examples/specification/run/binary-schema-nested-anonymous-record-truncated-json/`
  checks truncation JSON for a primitive inside that nested anonymous record.

## Remaining Work

The broader schema declaration surface proposal remains open for generated
runtime helper bindings outside the implemented binary helper boundaries and
format-neutral helper boundaries. Anonymous record encode support in
`format binary` schemas remains outside this completed decode-only slice.
Recursive or arbitrary-depth binary anonymous record decode also remains
outside this bounded slice.
