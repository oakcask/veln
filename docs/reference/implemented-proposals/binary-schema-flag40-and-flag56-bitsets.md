# Binary Schema Flag40 And Flag56 Bitsets

Status: implemented

This record preserves the former `Flag40be`, `Flag40le`, `Flag56be`, and
`Flag56le` visible flag bitset slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. That slice was
superseded by
[integer bitwise operators and flag removal](integer-bitwise-operators-and-flag-removal.md).
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/names-effects.md`, `../../specification/execution.md`,
and the checked executable examples under
`../../../examples/specification/run/`.

## Historical Outcome

`Flag40be`, `Flag40le`, `Flag56be`, and `Flag56le` are accepted as
`format binary` schema field primitive names for opt-in visible flag bitsets.
Generated decode helpers consume exactly five or seven bytes through the
matching `UInt40be`, `UInt40le`, `UInt56be`, or `UInt56le` representation path
and expose source-visible `FlagNNxx(bits: Int)` values instead of raw `Int`
fields. Generated encode helpers accept those flag values, emit exactly five
or seven bytes in the declared byte order, and reject negative or out-of-range
`bits` values through the existing `codec.encode_value_unrepresentable` path.

Pure prelude helpers expose checked bit access for indexes `0` through `39`
for `Flag40be` and `Flag40le`, and indexes `0` through `55` for `Flag56be`
and `Flag56le`. Raw-bit construction accepts only values in the matching
five-byte or seven-byte unsigned range. Direct structural decode and encode
mappings carry these flag fields with the same schema-local value shape as the
surrounding flag helper families.

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

The former flag family is historical only. Current schema code uses the
corresponding `uint...` primitive and represents decoded values as `Int`.
