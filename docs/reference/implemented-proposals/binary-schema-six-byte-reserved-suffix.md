# Binary Schema Six-Byte Reserved Suffix

Status: implemented

This record preserves the completed six-byte reserved suffix slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept a visible big-endian
sub-byte `UIntN` field followed immediately by
`ReservedBits(width, value)` when the two widths complete one six-byte
big-endian storage unit. The suffix form is bounded to visible widths one
through seven and reserved widths forty-one through forty-seven.

Decode reads the shared six-byte storage unit, returns the visible high bits as
an ordinary `Int`, validates the declared low reserved bits at the reserved
field path, omits the reserved field from decoded records and mapping source
values, and advances by six bytes. Encode omits the reserved field from the
input record, writes the visible high bits plus the declared low reserved bits
into the same six-byte storage unit, and reports
`codec.encode_value_unrepresentable` at the visible field path when the visible
value is outside its source-declared range.

The layout remains eligible for generated decode-step helpers and derived
codec decode and encode boundaries when the surrounding schema is otherwise
eligible.

## Evidence

- `../../../examples/specification/run/binary-schema-six-byte-reserved-suffix-decode-encode/`
  checks successful helper decode and encode, omitted reserved fields, and
  shared six-byte storage advancement.
- `../../../examples/specification/run/binary-schema-six-byte-reserved-suffix-json/`
  checks the JSON `schema.reserved_bits_mismatch` diagnostic with the reserved
  field path, bit width, expected value, actual value, and byte preview.
- `../../../examples/specification/run/binary-schema-six-byte-reserved-suffix-truncated-json/`
  checks the shared-storage truncation diagnostic for incomplete input.
- `../../../examples/specification/run/binary-schema-six-byte-reserved-suffix-encode-out-of-range/`
  checks the existing encode value-range diagnostic at the visible field path.
- `../../../examples/specification/run/derived-codec-six-byte-reserved-suffix-boundary/`
  checks the derived codec decode and encode boundary for the same layout.
- `crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks generated
  helper eligibility for all six-byte reserved suffix widths and rejection of
  unsupported adjacent shapes.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit layouts larger than the implemented helper slices, broader
dispatch forms, primitive shapes, and mapping behavior outside the implemented
generated-helper boundaries.
