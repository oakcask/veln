# Binary Schema Packed Visible Five-Byte Groups

Status: implemented

This record preserves the completed visible-only packed sub-byte five-byte
group slice from `../../proposals/binary-schema-primitives-and-dispatch.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept consecutive visible
`UInt1` through `UInt7` fields when at least two fields complete exactly one
five-byte big-endian storage unit. The fields are packed in declaration order
from high bits to low bits, and every visible field remains an ordinary `Int`
in decoded records, encoder inputs, structural mappings, generated
decode-step helpers, and explicit schema decode and encode expressions.

Truncation reports `schema.truncated_field` at the first field in the packed
group with a five-byte expected count. Encode range failures keep the existing
`codec.encode_value_unrepresentable` shape at the schema-local visible field
path.

## Evidence

- `../../../examples/specification/run/binary-schema-packed-visible-five-byte-decode-encode/`
  checks direct helper decode and encode, generated decode-step helper
  eligibility, explicit schema decode and encode expressions, and lowercase
  hex output.
- `../../../examples/specification/run/binary-schema-packed-visible-five-byte-truncated-json/`
  checks JSON truncation details for a short five-byte packed visible group.
- `../../../examples/specification/run/binary-schema-packed-visible-five-byte-encode-out-of-range/`
  checks visible field encode range failure projection on the schema-local
  field path.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
eight-byte visible-only packed groups plus reserved-bit layouts, dispatch
forms, primitive shapes, and mapping behavior outside the implemented
generated-helper slices.
