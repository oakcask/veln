# Binary Schema One-Byte Reserved Suffix

Status: implemented

This record preserves the completed one-byte reserved suffix slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept one visible
big-endian sub-byte `UIntN` field followed immediately by
`ReservedBits(width, value)` when the two widths complete one byte. The
completed target is the narrow `UInt7` plus `ReservedBits(1, 0)` layout.

Decode reads the shared one-byte storage unit, returns the visible high bits as
an ordinary `Int`, validates the declared low reserved bit at the reserved
field path, omits the reserved field from decoded records and mapping source
values, and advances by one byte. Encode omits the reserved field from the
input record, writes the visible high bits plus the declared low reserved bit
into the same byte, and reports `codec.encode_value_unrepresentable` at the
visible field path when the visible value is outside its declared range.

The layout remains eligible for generated decode-step helpers and derived
codec decode and encode boundaries when the surrounding schema is otherwise
eligible.

## Evidence

- `../../../examples/specification/run/binary-schema-one-byte-reserved-suffix-decode-encode/`
  checks successful helper decode and encode, omitted reserved fields, shared
  one-byte storage advancement, derived codec eligibility, and encode range
  failure at the visible field path.
- `../../../examples/specification/run/binary-schema-one-byte-reserved-suffix-json/`
  checks the JSON `schema.reserved_bits_mismatch` diagnostic with the reserved
  field path, bit width, expected value, actual value, and byte preview.
- `crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks generated
  helper eligibility for the isolated one-byte reserved suffix field pair.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit layouts outside the implemented helper slices, broader dispatch
forms, primitive shapes, and mapping behavior outside the implemented
generated-helper boundaries. This record does not extend the one-byte suffix
slice beyond the completed field pair.
