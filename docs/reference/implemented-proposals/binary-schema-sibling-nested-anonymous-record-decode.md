# Binary Schema Sibling Nested Anonymous Record Decode

Status: implemented

This record preserves the completed generated decode helper slice for binary
schema anonymous record fields that contain more than one nested anonymous
record field at the same record level. Current behavior is specified by
`../../specification/source-surface.md`, `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode helpers accept an anonymous record field with
sibling nested anonymous record fields when every leaf is an implemented
exact-width unsigned primitive. Decode keeps source-order leaf consumption,
the schema-local visible record shape, and consumed-byte accounting for both
the compatibility helper and explicit `decode <Schema> from <view> at
<offset>` operation.

Runtime truncation inside the second nested anonymous record sibling uses the
existing `schema.truncated_field` byte diagnostic shape. The field path keeps
the outer schema field segment and appends the selected sibling field and
failed primitive field segments, without inserting a synthetic nested schema
segment.

This decode-only record does not define encode helper behavior. Current
anonymous record encode behavior is covered by
`binary-schema-anonymous-record-encode.md`.

## Evidence

- `../../../examples/specification/run/binary-schema-sibling-nested-anonymous-record-decode/`
  checks successful decode through the generated helper and explicit schema
  decode operation for an anonymous record with two nested anonymous record
  siblings.
- `../../../examples/specification/run/binary-schema-sibling-nested-anonymous-record-truncated-json/`
  checks truncation JSON for a primitive inside the second nested anonymous
  record sibling, including field path, byte offset, expected count,
  available count, and readiness.

## Boundary

Schema composition is complete across the implemented helper boundaries.
Additional anonymous-record leaf families and new binary field families are
non-goals for this record and require separate focused proposals.
