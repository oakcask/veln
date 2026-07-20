# Binary Schema Anonymous Record Encode

Status: implemented

This record preserves the completed generated encode helper slice for binary
schema fields whose type is an anonymous record made from implemented
exact-width unsigned primitive leaves. Current behavior is specified by
`../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema encode helpers accept schema-local visible records
containing anonymous record fields when every anonymous record leaf is an
implemented exact-width unsigned primitive. Encode writes the anonymous record
leaves in declaration order, preserves each primitive's byte order, and emits
the same byte layout read by the existing anonymous record decode helper.

Runtime representability failures inside the anonymous record use the existing
`schema.encode_value_unrepresentable` value diagnostic shape. The field path
keeps the outer schema field segment and appends each anonymous record field
segment down to the failed primitive, without inserting a synthetic nested
schema segment.

Unsupported anonymous record leaves remain outside generated binary schema
encode helper eligibility. They do not receive partial runtime support through
the anonymous record encode path.

## Evidence

- `../../../examples/specification/run/binary-schema-anonymous-record-encode/case.toml`
  checks successful generated encode for a visible anonymous record field with
  sibling nested anonymous record fields and exact-width unsigned primitive
  leaves.
- `../../../examples/specification/run/binary-schema-anonymous-record-encode-out-of-range-json/case.toml`
  checks JSON projection for an out-of-range nested leaf value, including the
  nested schema-local field path and source-visible display path.
- `../../../examples/specification/check/binary-schema-anonymous-record-encode-boundary/case.toml`
  keeps an unsupported nested anonymous record leaf outside generated encode
  helper eligibility.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  generated helper resolution and nested metadata for the anonymous record
  encode boundary.

## Boundary

Schema composition is complete across the implemented helper boundaries.
Unsupported anonymous-record leaves and new binary field families are
non-goals for this record and require separate focused proposals.
