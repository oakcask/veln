# Binary Schema Flag40 And Flag56 Bitsets

Status: implemented

This record preserves the completed `Flag40be`, `Flag40le`, `Flag56be`, and
`Flag56le` visible flag bitset slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/names-effects.md`, `../../specification/execution.md`,
and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

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

## Evidence

- `../../../examples/specification/run/binary-schema-flag40be-decode/`,
  `../../../examples/specification/run/binary-schema-flag40le-decode/`,
  `../../../examples/specification/run/binary-schema-flag56be-decode/`, and
  `../../../examples/specification/run/binary-schema-flag56le-decode/` check
  five-byte and seven-byte decode into visible flag values.
- `../../../examples/specification/run/binary-schema-flag40be-encode/`,
  `../../../examples/specification/run/binary-schema-flag40le-encode/`,
  `../../../examples/specification/run/binary-schema-flag56be-encode/`, and
  `../../../examples/specification/run/binary-schema-flag56le-encode/` check
  five-byte and seven-byte encode output.
- The matching `binary-schema-flag40*-mapped-record-*` and
  `binary-schema-flag56*-mapped-record-*` executable examples check direct
  structural mappings in both directions.
- `../../../examples/specification/run/binary-schema-flag40be-bit-helpers/`,
  `../../../examples/specification/run/binary-schema-flag40le-bit-helpers/`,
  `../../../examples/specification/run/binary-schema-flag56be-bit-helpers/`,
  and `../../../examples/specification/run/binary-schema-flag56le-bit-helpers/`
  check successful helper reads, writes, raw-bit extraction, raw-bit
  construction, and generated encode use.
- The matching `from-bits-out-of-range`, `bit-index-json`, and
  `bit-index-human` executable examples check helper failure reporting.
- `../../../examples/specification/run/binary-schema-flag40be-encode-out-of-range/`,
  `../../../examples/specification/run/binary-schema-flag40le-encode-out-of-range/`,
  `../../../examples/specification/run/binary-schema-flag56be-encode-out-of-range/`,
  and `../../../examples/specification/run/binary-schema-flag56le-encode-out-of-range/`
  check generated encode range failures.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
flag vocabulary and mapping behavior outside the implemented generated-helper
slices.
