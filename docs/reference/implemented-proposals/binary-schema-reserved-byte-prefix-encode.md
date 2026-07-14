# Binary Schema Reserved Byte Prefix Encode

Status: superseded

This record preserves the completed reserved-byte-prefix encode slice from
`binary-schema-primitives-and-dispatch.md`. The general rule
that replaced this narrow slice is recorded in
`binary-schema-general-reserved-byte-prefixes.md`; current behavior is
specified under `../../specification/`.

## Outcome

At the time of this slice, generated binary schema decode and encode helpers
accepted the narrow
`ReservedBits(2, 0)` and `ReservedBits(9, 0)` followed by `UInt8` layouts as
two-byte big-endian bitstream slices. The reserved field is
representation-only: it is omitted from decoded result records, encoder value
records, and mapping source values.

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
  failure behavior for `ReservedBits(2, 0)` followed by `UInt8`.
- `../../../examples/specification/run/binary-schema-reserved-nine-bit-prefix-decode-encode/`
  checks the same helper and derived codec route for `ReservedBits(9, 0)`
  followed by `UInt8`; the adjacent JSON cases check truncation and
  reserved-bit mismatch details.
- `crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks helper
  eligibility for the accepted reserved-byte-prefix layouts and rejection for
  unsupported reserved-bit encode groups.

## Superseded By

`binary-schema-general-reserved-byte-prefixes.md` replaces the width list with
one bounded rule and contains the current completion evidence.
