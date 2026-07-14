# Binary Schema Big-Endian Width Parity

Status: implemented

This record preserves the completed direct visible big-endian exact-width
primitive helper parity slices from
`binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Direct visible `UInt16be`, `UInt24be`, `UInt31be`, `UInt32be`, `UInt56be`,
and `UInt64be` fields in plain `format binary` schemas expose ordinary
generated `byte_decode_<schema>` and `byte_encode_<schema>` helper behavior.
Decode helpers consume the declared big-endian byte widths and expose ordinary
`Int` values. Encode helpers accept ordinary `Int` fields, emit the declared
big-endian byte widths, and reject negative or out-of-range values through
`schema.encode_value_unrepresentable`.

The `UInt31be` primitive keeps the high-order bit outside the representable
visible value range: `2147483647` is accepted and `2147483648` is rejected.
The `UInt56be` primitive accepts up to `72057594037927935`. The `UInt64be`
helper exposes ordinary `Int` fields; negative input is rejected through the
same generated encode error path.

## Evidence

- `../../../examples/specification/run/binary-schema-big-endian-widths-decode-encode/`
  checks one generated schema helper pair containing `UInt16be`, `UInt24be`,
  `UInt31be`, and `UInt32be` direct visible fields, including the maximum
  valid `UInt31be` value.
- `../../../examples/specification/run/binary-schema-big-endian-widths-encode-out-of-range/`
  checks the generated `schema.encode_value_unrepresentable` range failure
  for an out-of-range `UInt31be` value.
- `../../../examples/specification/run/binary-schema-u56-widths-encode/` and
  `../../../examples/specification/run/binary-schema-u64-widths-encode/`
  check wide big-endian generated encode output and generated decode helper
  `Int` values.
- `../../../examples/specification/run/binary-schema-u56-widths-encode-out-of-range/`
  and
  `../../../examples/specification/run/binary-schema-u64-widths-encode-out-of-range/`
  check generated encode failures for unrepresentable wide values.
- `../../../examples/specification/run/binary-schema-u64-widths-integer-out-of-range-human/`
  and
  `../../../examples/specification/run/binary-schema-u64-widths-integer-out-of-range-json/`
  check generated decode range diagnostics for eight-byte unsigned values
  outside the visible `Int` range.
- `../../../examples/specification/run/binary-schema-u56-widths-truncated-json/`
  and
  `../../../examples/specification/run/binary-schema-u64-widths-truncated-json/`
  check wide generated decode truncation diagnostics.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
primitive widths, dispatch forms, reserved-bit groups, repeats, byte views,
and runtime diagnostics outside this implemented generated-helper slice.
