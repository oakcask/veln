# Binary Schema Byte-Visible Reserved Suffix

Status: implemented

This record preserves the completed `UInt8` plus multi-byte reserved suffix
slice from `../../proposals/binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept a visible `UInt8`
field followed immediately by a non-byte-aligned multi-byte
`ReservedBits(width, value)` suffix when the visible byte, the reserved suffix,
and zero low padding bits fit in one three-byte through eight-byte big-endian
storage unit. This covers the `UInt8` followed by `ReservedBits(9, 0)` layout
that previously reported `schema.reserved_bits_encode`.

Decode reads the shared storage unit, returns the visible byte as an ordinary
`Int`, validates the declared reserved suffix at the reserved field path,
omits the reserved field from decoded records and mapping source values unless
an explicit mapping assignment names it, and advances by the shared storage
width. Encode omits the reserved field from the input record, writes the
visible byte followed by the declared reserved value and zero low padding bits,
and reports `codec.encode_value_unrepresentable` at the visible field path
when the visible value is outside the `UInt8` range.

The layout remains eligible for generated decode-step helpers and derived
codec decode and encode boundaries when the surrounding schema is otherwise
eligible.

## Evidence

- `../../../examples/specification/run/binary-schema-byte-visible-reserved-suffix-decode-encode/`
  checks successful helper decode and encode, omitted reserved fields, shared
  three-byte storage advancement, derived codec decode and encode, and the
  visible-field encode range diagnostic.
- `../../../examples/specification/run/binary-schema-byte-visible-reserved-suffix-json/`
  checks the JSON `schema.reserved_bits_mismatch` diagnostic with the reserved
  field path, bit width, expected value, actual value, and byte preview.
- `../../../examples/specification/check/schema-reserved-bit-layout-diagnostics/`
  and `../../../examples/specification/check/schema-reserved-bit-layout-human/`
  keep adjacent unsupported reserved-bit layout diagnostics while no longer
  reporting the `UInt8` plus `ReservedBits(9, 0)` suffix as unsupported.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit layouts outside the implemented bounded storage-unit rules,
broader dispatch forms, primitive shapes, and mapping behavior outside the
implemented generated-helper boundaries.
