# Binary Schema Reserved Byte Prefix Encode

Status: implemented

This record preserves the completed reserved-byte-prefix encode slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept the narrow
`ReservedBits(2, 0)` followed by `UInt8` layout as a two-byte big-endian
bitstream slice. The reserved field is representation-only: it is omitted from
decoded result records, encoder value records, and mapping source values.

Decode validates the declared high reserved bits, decodes the following visible
byte into an ordinary `Int` field, ignores the low padding bits, and advances by
two bytes. Encode emits the declared reserved bits first, then the visible
`UInt8` value, then zero low padding bits, producing deterministic lowercase
hex output through the existing byte-chunk reporting path.

Visible-field encode range failures keep the existing
`codec.encode_value_unrepresentable` shape at the visible field path. Other
non-byte-aligned reserved-bit encode layouts outside the implemented helper
slices continue to report `schema.reserved_bits_encode` during `check`.

## Evidence

- `../../../examples/specification/run/binary-schema-reserved-byte-prefix-decode-encode/`
  checks direct helper decode and encode, derived codec decode and encode,
  lowercase hex output, omitted reserved fields, and visible-field range
  failure behavior.
- `../../../examples/specification/check/schema-reserved-bit-encode-diagnostics/`
  checks that an adjacent unsupported non-byte-aligned reserved-bit encode
  layout still reports `schema.reserved_bits_encode`.
- `crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks helper
  eligibility for the accepted reserved-byte-prefix layout and rejection for
  unsupported reserved-bit encode groups.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit layouts, dispatch forms, primitive shapes, and mapping behavior
outside the implemented generated-helper slices.
