# Binary Schema ByteView Payload Multiple

Status: implemented

This record preserves the completed schema-owned `ByteView` payload multiple
validation slice from `binary-schema-primitives-and-dispatch.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode helpers accept
`where payload_count multiple of field_name` on a length-bounded `ByteView`
field when `field_name` names an earlier decoded visible `Int` field in the
same schema. They also accept
`where payload_count multiple of positive_integer` for positive integer
literal multiples.

The dependency is representation-local. Multiple operands cannot refer to
protocol state, settings, arbitrary calls, later fields, unknown fields, or
fields decoded as non-`Int` values. Unsupported operands and uses on
non-`ByteView` fields report `schema.byte_view_reference` during checking.

Decode helpers validate the computed payload count after the length expression
has been checked for bounds. Mismatches report
`schema.length_multiple_mismatch` with schema field path, byte offset,
observed payload count, required multiple, operand, and bounded byte preview
details.

## Evidence

- `../../../examples/specification/run/binary-schema-byteview-multiple-decode/`
  checks a successful generated helper decode using an earlier decoded `Int`
  field as the multiple operand.
- `../../../examples/specification/run/binary-schema-byteview-multiple-json/`
  checks structured `run --json` projection for
  `schema.length_multiple_mismatch`.
- `../../../examples/specification/run/binary-schema-byteview-multiple-human/`
  checks focused human diagnostic projection for a literal multiple mismatch.
- Parser, formatter, semantic, source-surface fixture, and executable grammar
  coverage pin the accepted source form and declaration-time rejection
  boundary.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
general schema-declared payload structures, dispatch-specific payload multiple
checks, encode-side representability checks, protocol-state rules, and
dependencies outside the representation-local generated-helper boundary.
