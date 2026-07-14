# Binary Schema Split Reserved Groups

Status: implemented

This record preserves the completed consecutive split reserved group slice
from `binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept consecutive
non-byte-aligned `UIntN` and `ReservedBits(width, value)` fields whose
declared widths complete one byte or the same two-byte, three-byte,
four-byte, five-byte, six-byte, seven-byte, or eight-byte big-endian storage
unit. The group must contain at least one visible field and at least one
reserved field, and every visible field remains a big-endian sub-byte
`UIntN`.

Reserved fields are representation-only. They are omitted from decoded
records, mapping source values, and encoder value records. Decode validates
each reserved value at its declared field path and returns visible fields in
declaration order. Encode writes visible and reserved values in declaration
order and reports `codec.encode_value_unrepresentable` at the out-of-range
visible field.

The layout remains eligible for generated decode-step helpers and derived
codec boundaries that accept the same generated helper slice.

## Evidence

- `../../../examples/specification/run/binary-schema-split-reserved-decode-encode/`
  checks the one-byte decode and encode layout, reserved-field omission from
  the value record, and visible-field encode range failure.
- `../../../examples/specification/run/binary-schema-five-byte-split-reserved-decode-encode/`
  and
  `../../../examples/specification/run/binary-schema-six-byte-split-reserved-decode-encode/`
  check larger shared-storage decode and encode layouts.
- `../../../examples/specification/run/binary-schema-five-byte-split-reserved-json/`
  and
  `../../../examples/specification/run/binary-schema-six-byte-split-reserved-json/`
  check JSON `schema.reserved_bits_mismatch` diagnostics with field path, bit
  width, expected value, actual value, and byte preview details.
- `../../../examples/specification/run/binary-schema-five-byte-split-reserved-human/`
  and
  `../../../examples/specification/run/binary-schema-six-byte-split-reserved-human/`
  check the matching human diagnostic projection.
- `../../../examples/specification/run/derived-codec-split-reserved-boundary/`
  checks derived decode and encode eligibility for the shared helper path.
- The seven-byte and eight-byte companion records keep focused evidence for
  those wider layouts.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
broader non-byte-aligned shapes, new dispatch forms, repeat forms, and mapping
behavior outside the implemented generated-helper slices.
