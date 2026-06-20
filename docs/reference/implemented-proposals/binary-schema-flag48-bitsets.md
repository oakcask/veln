# Binary Schema Flag48 Bitsets

Status: implemented

This record preserves the completed `Flag48be` and `Flag48le` visible flag
bitset slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/names-effects.md`, `../../specification/execution.md`,
and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

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

## Evidence

- `../../../examples/specification/run/binary-schema-flag48be-decode/` and
  `../../../examples/specification/run/binary-schema-flag48le-decode/` check
  six-byte big-endian and little-endian decode into visible flag values.
- `../../../examples/specification/run/binary-schema-flag48be-encode/` and
  `../../../examples/specification/run/binary-schema-flag48le-encode/` check
  six-byte big-endian and little-endian encode output.
- `../../../examples/specification/run/binary-schema-flag48be-mapped-record-decode/`,
  `../../../examples/specification/run/binary-schema-flag48le-mapped-record-decode/`,
  `../../../examples/specification/run/binary-schema-flag48be-mapped-record-encode/`,
  and `../../../examples/specification/run/binary-schema-flag48le-mapped-record-encode/`
  check direct structural mappings in both directions.
- `../../../examples/specification/run/binary-schema-flag48be-bit-helpers/`
  and `../../../examples/specification/run/binary-schema-flag48le-bit-helpers/`
  check successful helper reads, writes, raw-bit extraction, raw-bit
  construction, and generated encode use.
- `../../../examples/specification/run/binary-schema-flag48be-from-bits-out-of-range-json/`,
  `../../../examples/specification/run/binary-schema-flag48le-from-bits-out-of-range-json/`,
  `../../../examples/specification/run/binary-schema-flag48be-bit-index-json/`,
  `../../../examples/specification/run/binary-schema-flag48le-bit-index-json/`,
  `../../../examples/specification/run/binary-schema-flag48be-bit-index-human/`,
  and `../../../examples/specification/run/binary-schema-flag48le-bit-index-human/`
  check helper failure reporting.
- `../../../examples/specification/run/binary-schema-flag48be-encode-out-of-range/`
  and `../../../examples/specification/run/binary-schema-flag48le-encode-out-of-range/`
  check generated encode range failures.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
flag widths and mapping behavior outside the implemented generated-helper
slices.
