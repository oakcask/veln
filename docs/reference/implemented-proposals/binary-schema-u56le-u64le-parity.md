# Binary Schema UInt56le And UInt64le Parity

Status: implemented

This record preserves the completed direct visible `UInt56le` and `UInt64le`
generated helper parity slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked executable examples under
`../../../examples/specification/run/` and
`../../../examples/specification/check/`.

## Outcome

Direct visible `UInt56le` and `UInt64le` fields in plain `format binary`
schemas expose ordinary generated `byte_decode_<schema>` and
`byte_encode_<schema>` helper behavior. Decode helpers consume the declared
little-endian byte widths and expose ordinary `Int` values. Encode helpers
accept ordinary `Int` fields, emit the declared little-endian byte widths, and
reject negative or out-of-range values through
`schema.encode_value_unrepresentable`.

The `UInt56le` primitive accepts up to `72057594037927935`. The `UInt64le`
decode helper rejects unsigned eight-byte values above the source-visible
`Int` maximum with `schema.integer_out_of_range`, preserving the schema-local
field path, byte width, integer bounds, actual value text, and byte preview.
Truncated decode input keeps the shared `schema.truncated_field` diagnostic
shape, including byte offset, field path, requested byte count, available byte
count, readiness, and bounded byte-preview details.

## Evidence

- `../../../examples/specification/run/binary-schema-u56-widths-encode/` and
  `../../../examples/specification/run/binary-schema-u64-widths-encode/`
  check wide little-endian generated encode output and generated decode helper
  `Int` values.
- `../../../examples/specification/run/binary-schema-u56-widths-encode-out-of-range/`
  and
  `../../../examples/specification/run/binary-schema-u64-widths-encode-out-of-range/`
  check generated encode failures for unrepresentable wide little-endian
  values.
- `../../../examples/specification/run/binary-schema-u64le-widths-integer-out-of-range-json/`
  checks the `UInt64le` generated decode range diagnostic for an eight-byte
  unsigned value outside the visible `Int` range.
- `../../../examples/specification/run/binary-schema-u56-widths-truncated-json/`
  and
  `../../../examples/specification/run/binary-schema-u64-widths-truncated-json/`
  check wide little-endian generated decode truncation diagnostics.
- `../../../examples/specification/check/schema-exact-width-primitive-diagnostics/`
  checks that `UInt56le` and `UInt64le` remain schema-local names.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
primitive widths, dispatch forms, reserved-bit groups, repeats, byte views,
and runtime diagnostics outside this implemented generated-helper slice.
