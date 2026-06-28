# Binary Schema Wide Reserved Suffix Groups

Status: implemented

This record preserves the completed seven-byte and eight-byte reserved suffix
group slice from
[../../proposals/binary-schema-primitives-and-dispatch.md](../../proposals/binary-schema-primitives-and-dispatch.md).
Current behavior is specified by
[../../specification/source-surface.md](../../specification/source-surface.md),
[../../specification/execution.md](../../specification/execution.md), and the
checked executable examples under `../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept a visible big-endian
sub-byte `UIntN` field followed immediately by
`ReservedBits(width, value)` when the two declared widths complete one shared
seven-byte or eight-byte big-endian storage unit. The seven-byte form accepts
reserved suffix widths forty-nine through fifty-five, and the eight-byte form
accepts reserved suffix widths fifty-seven through sixty-three. These forms
close the bounded single-visible reserved-suffix extension at the implemented
exact-width schema storage limit of eight bytes.

Reserved suffix fields are representation-only. Decode reads the shared
storage unit, decodes the visible field from the high bits, validates the
declared low reserved bits at the reserved field path, omits the reserved
field from decoded records and mapping source values, and advances by the
shared storage width. Encode omits the reserved field from the input record,
writes the visible value followed by the declared reserved bits, and reports
`codec.encode_value_unrepresentable` at the out-of-range visible field.

The layout remains eligible for generated decode-step helpers and derived
codec decode and encode boundaries when the surrounding schema is otherwise
eligible.

## Evidence

- `../../../examples/specification/run/binary-schema-wide-suffix-reserved-seven-byte-decode-encode/`
  checks successful seven-byte helper decode and encode, omitted reserved
  fields, derived codec decode and encode eligibility, shared storage
  advancement, and encode range failure at the visible field path.
- `../../../examples/specification/run/binary-schema-wide-suffix-reserved-eight-byte-decode-encode/`
  checks the matching eight-byte helper behavior.
- `../../../examples/specification/run/binary-schema-wide-suffix-reserved-json/`
  checks JSON `schema.reserved_bits_mismatch` diagnostics with field path, bit
  width, expected value, actual value, and byte preview details.
- `../../../examples/specification/run/binary-schema-wide-suffix-reserved-human/`
  checks the matching human diagnostic projection.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
non-byte-aligned shapes outside the implemented helper slices, dispatch forms,
repeat forms, and mapping behavior outside the implemented generated helper
boundaries. Reserved-suffix groups larger than eight bytes are a non-goal for
this completed slice.
