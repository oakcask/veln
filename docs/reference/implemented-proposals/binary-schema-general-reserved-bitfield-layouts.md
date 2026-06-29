# Binary Schema General Reserved Bitfield Layouts

Status: implemented

This record preserves the completed general reserved bitfield layout slice
from `../../proposals/binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, and checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers use one shared big-endian
bitfield layout model for consecutive non-byte-aligned `UIntN` and
`ReservedBits(width, value)` fields when the group contains at least one
visible field and at least one reserved field and the declared widths complete
one supported storage unit.

The layout model is not limited to named prefix, suffix, middle, split, or
byte-visible reserved slices. It validates field widths before helper
generation, decodes and encodes visible fields from their declared bit
positions, validates each reserved field at its own field path, emits fixed
reserved bits during encode, omits reserved fields from decoded and encoder
value records, and preserves `codec.encode_value_unrepresentable` for visible
field range failures.

## Evidence

- `../../../examples/specification/run/binary-schema-general-reserved-bitfield-decode-encode/`
  checks a two-byte layout that starts and ends with visible fields and
  contains multiple reserved fields. It covers decode success, direct encode
  success, derived decode and encode eligibility, and encode range failure.
- `../../../examples/specification/run/binary-schema-general-reserved-bitfield-json/`
  checks reserved-bit decode failure with the standard
  `schema.reserved_bits_mismatch` JSON projection.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
dispatch, repeat, mapping, protocol, and schema-value behavior outside the
implemented generated-helper slices.
