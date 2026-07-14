# Binary Schema Flag48 Bitsets

Status: implemented

This record preserves the former `Flag48be` and `Flag48le` visible flag bitset
slice from `binary-schema-primitives-and-dispatch.md`. That
slice was superseded by
[integer bitwise operators and flag removal](integer-bitwise-operators-and-flag-removal.md).
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/names-effects.md`, `../../specification/execution.md`,
and the checked executable examples under
`../../../examples/specification/run/`.

## Historical Outcome

`Flag48be` and `Flag48le` are accepted as `format binary` schema field
primitive names for opt-in visible flag bitsets. Generated decode helpers
consume exactly six bytes through the existing `UInt48be` or `UInt48le`
representation path and expose source-visible `Flag48be(bits: Int)` or
`Flag48le(bits: Int)` values instead of raw `Int` fields. Generated encode
helpers accept those flag values, emit exactly six bytes in the declared byte
order, and reject negative or out-of-range `bits` values through the existing
`codec.encode_value_unrepresentable` path.

Pure prelude helpers expose checked bit access for indexes `0` through `47`
and checked raw-bit construction for values in the six-byte unsigned range.
Direct structural decode and encode mappings carry `Flag48be` and `Flag48le`
fields with the same schema-local value shape as the surrounding flag helper
families.

## Superseding Evidence

- `../../../examples/specification/run/binary-schema-uint-bit-operations-both-byte-orders/`
  checks the replacement exact-width unsigned fields and ordinary integer bit
  operations in both byte orders.
- `../../../examples/specification/check/removed-flag-vocabulary-diagnostics/`
  and
  `../../../examples/specification/check/removed-flag-nested-shapes-human/`
  check removal diagnostics and replacement names.
- The superseding implemented proposal record preserves the migration map and
  the reason the original dedicated flag fixtures were removed.

## Supersession

The former flag family is historical only. Current schema code uses
`uint48be` or `uint48le` and represents decoded values as `Int`.
