# Binary Schema Wide Reserved Prefix Groups

Status: implemented

This record preserves the completed seven-byte and eight-byte reserved prefix
group slice from `binary-schema-primitives-and-dispatch.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept
`ReservedBits(width, value)` followed by two visible big-endian sub-byte or
byte-width `UIntN` fields when the three declared widths complete one shared
seven-byte or eight-byte big-endian storage unit. The seven-byte form accepts
reserved prefix width forty-nine, and the eight-byte form accepts reserved
prefix width fifty-seven. These forms close the bounded reserved-prefix group
extension at the implemented exact-width schema storage limit of eight bytes.

Reserved prefix fields are representation-only. Decode validates the declared
reserved value at the reserved field path, decodes the two visible fields from
high to low in declaration order, omits the reserved field from decoded records
and mapping source values, and advances by the shared storage width. Encode
omits the reserved field from the input record, writes the declared high
reserved bits followed by the two visible fields, and reports
`codec.encode_value_unrepresentable` at the out-of-range visible field.

## Evidence

- `../../../examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-decode-encode/`
  checks successful seven-byte helper decode and encode, omitted reserved
  fields, declaration-order visible fields, and shared storage advancement.
- `../../../examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-decode-encode/`
  checks the matching eight-byte helper behavior.
- `../../../examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-json/`
  and
  `../../../examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-json/`
  check JSON `schema.reserved_bits_mismatch` diagnostics with field path, bit
  width, expected value, actual value, and byte preview details.
- `../../../examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-human/`
  and
  `../../../examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-human/`
  check the matching human diagnostic projection.
- `../../../examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-encode-out-of-range/`
  and
  `../../../examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-encode-out-of-range/`
  check the existing encode value-range diagnostic at the visible field path.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
non-byte-aligned shapes outside the implemented helper slices, new dispatch
forms, repeat forms, and mapping behavior outside the implemented generated
helper boundaries. Reserved-prefix groups larger than eight bytes are a
non-goal for this completed slice.
