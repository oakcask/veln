# Binary Schema Reserved Fifteen-Bit Prefix

Status: implemented

This record preserves the completed two-field reserved-prefix boundary from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept
`ReservedBits(15, value)` followed immediately by visible `UInt1` when the two
fields complete one two-byte big-endian storage unit. The reserved field is
representation-only: it is omitted from decoded result records, encoder value
records, and mapping source values.

Decode validates the declared high reserved bits, decodes the low visible bit
into an ordinary `Int` field, and advances by two bytes. Encode emits the
declared reserved bits first and then the visible `UInt1` value. Generated
decode-step helpers and derived codecs remain eligible for the layout.

Reserved-bit mismatches keep the existing `schema.reserved_bits_mismatch`
shape. Visible-field encode range failures keep the existing
`codec.encode_value_unrepresentable` shape at the schema-local visible field
path. Other unsupported `ReservedBits` layouts remain outside this slice.

## Evidence

Helper eligibility for the accepted two-byte packed reserved-prefix width
range, including the width-fifteen boundary, remains covered by semantic tests.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit layouts, dispatch forms, primitive shapes, and mapping behavior
outside the implemented generated-helper slices.
