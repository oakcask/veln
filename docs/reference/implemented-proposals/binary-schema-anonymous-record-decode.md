# Binary Schema Anonymous Record Decode

Status: implemented

This record preserves the completed generated decode helper slice for binary
schema fields whose type is an anonymous record made from implemented
exact-width unsigned primitive leaves, through the original single nested
anonymous record boundary. Current behavior is specified by
`../../specification/source-surface.md`, `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode helpers accept a visible field whose schema type
is an anonymous record. Each leaf must be an implemented exact-width unsigned
primitive. Decode reads the leaves in source order and exposes the same
anonymous record shape at the outer schema field. Later sibling nested
anonymous record support is archived under
`binary-schema-sibling-nested-anonymous-record-decode.md`.

Runtime truncation inside the anonymous record uses the existing
`schema.truncated_field` byte diagnostic shape. The field path keeps the outer
schema field segment and appends each anonymous record field segment down to
the failed primitive, without inserting a synthetic nested schema segment.

This decode-only record does not define encode helper behavior. Current
anonymous record encode behavior is covered by
`binary-schema-anonymous-record-encode.md`.

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
- `../../../examples/specification/run/binary-schema-recursive-anonymous-record-decode/`
  checks successful decode through another anonymous record layer.
- `../../../examples/specification/run/binary-schema-recursive-anonymous-record-truncated-json/`
  checks truncation JSON keeps every recursive anonymous record path segment.

## Boundary

Schema composition is complete across the implemented helper boundaries.
Unsupported anonymous-record leaves and new binary field families are
non-goals for this record and require separate focused proposals.
