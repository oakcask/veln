# Binary Schema Suffix Reserved Groups

Status: implemented

This record preserves the completed two-byte suffix reserved group slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept two visible
big-endian `UIntN` fields followed by a non-byte-aligned
`ReservedBits(width, value)` suffix when one visible field is `UInt8` and the
three declared widths complete one shared two-byte big-endian storage unit.

Decode reads the shared two-byte storage unit, decodes the visible fields from
their declared high-to-low positions as ordinary `Int` values, validates the
declared low reserved bits at the reserved field path, omits the reserved field
from decoded records and mapping source values, and advances by two bytes.
Encode omits the reserved field from the input record, writes the visible
values in declaration order followed by the declared low reserved bits, and
reports `codec.encode_value_unrepresentable` at the out-of-range visible field.

The layout is eligible for generated decode-step helpers and derived codec
decode and encode boundaries when the surrounding schema is otherwise eligible.

## Evidence

- `../../../examples/specification/run/binary-schema-suffix-reserved-group-decode-encode/`
  checks successful helper decode and encode, omitted reserved fields, shared
  two-byte storage advancement, derived codec eligibility, and encode range
  failure at the visible field path.
- `../../../examples/specification/run/binary-schema-suffix-reserved-group-json/`
  checks the JSON `schema.reserved_bits_mismatch` diagnostic with the reserved
  field path, bit width, expected value, actual value, and byte preview.
- `crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks generated
  helper eligibility for both supported field orders where one visible field is
  `UInt8`.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
non-byte-aligned shapes outside the implemented helper slices, new dispatch
forms, repeat forms, and mapping behavior outside the implemented generated
helper boundaries. Extending the suffix reserved group beyond the bounded
two-byte shape in this record is proposal work, not an implicit continuation of
this completed slice.
