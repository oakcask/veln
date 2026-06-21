# Binary Schema Seven-Byte Split Reserved Layouts

Status: implemented

This record preserves the completed seven-byte split reserved layout slice
from `../../proposals/binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept consecutive
non-byte-aligned `UIntN` and `ReservedBits(width, value)` groups whose
declared widths complete one seven-byte big-endian storage unit. The group
must contain at least one visible field and at least one reserved field, and
every visible field remains a big-endian sub-byte `UIntN`.

Reserved fields are representation-only. They are omitted from decoded
records, mapping source values, and encoder value records. Decode validates
each reserved value at its declared field path and returns visible fields in
declaration order. Encode writes visible and reserved values in declaration
order and reports `codec.encode_value_unrepresentable` at the out-of-range
visible field.

The layout remains eligible for the existing structural mapping,
generated decode-step helper, and derived codec boundaries that accept the
smaller shared-storage split reserved layouts.

## Evidence

- `../../../examples/specification/run/binary-schema-seven-byte-split-reserved-decode-encode/`
  checks decode, encode, reserved-field omission from the value record, and
  visible-field encode range failure.
- `../../../examples/specification/run/binary-schema-seven-byte-split-reserved-json/`
  checks the JSON `schema.reserved_bits_mismatch` diagnostic with field path,
  bit width, expected value, actual value, and byte preview details.
- `../../../examples/specification/run/binary-schema-seven-byte-split-reserved-human/`
  checks the matching human diagnostic projection.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
eight-byte or larger split reserved groups, broader non-byte-aligned shapes,
new dispatch forms, repeat forms, and mapping behavior outside the implemented
generated-helper slices.
