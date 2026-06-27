# Binary Schema Packed Visible Two-Byte Groups

Status: implemented

This record preserves the completed visible-only packed sub-byte two-byte
group slice from `../../proposals/binary-schema-primitives-and-dispatch.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept consecutive visible
`UInt1` through `UInt7` fields when at least two fields complete exactly one
two-byte big-endian storage unit. The fields are packed in declaration order
from high bits to low bits, and every visible field remains an ordinary `Int`
in decoded records, encoder inputs, structural mappings, generated decode-step
helpers, and derived codecs.

Truncation reports `schema.truncated_field` at the first field in the packed
group with a two-byte expected count. Encode range failures keep the existing
`codec.encode_value_unrepresentable` shape at the schema-local visible field
path.

## Evidence

- `../../../examples/specification/run/binary-schema-packed-visible-two-byte-decode-encode/`
  checks direct helper decode and encode, generated decode-step helper
  eligibility, derived codec decode and encode, and lowercase hex output.
- `../../../examples/specification/run/binary-schema-packed-visible-two-byte-truncated-json/`
  checks JSON truncation details for a short two-byte packed visible group.
- `../../../examples/specification/run/binary-schema-packed-visible-two-byte-encode-out-of-range/`
  checks visible field encode range failure projection on the schema-local
  field path.
- `../../../examples/specification/run/derived-codec-packed-visible-two-byte-boundary/`
  checks derived codec decode readiness, budgeted encode resume, and
  helper-projected encode failure boundaries for the same packed group.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit layouts, dispatch forms, primitive shapes, and mapping behavior
outside the implemented generated-helper slices.
