# Binary Schema UInt40 Primitives

Status: implemented

This record preserves the completed `UInt40be` and `UInt40le` exact-width
binary schema primitive slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/` and
`../../../examples/specification/check/`.

## Outcome

`UInt40be` and `UInt40le` are accepted only as `format binary` schema field
primitive names. Generated decode helpers consume exactly five bytes in the
declared byte order and expose visible fields as ordinary `Int` values.
Generated encode helpers accept ordinary `Int` fields, emit exactly five bytes
in the declared byte order, and reject negative or out-of-range values through
the existing exact-width `EncodeError` path.

Truncated decode input keeps the shared `schema.truncated_field` diagnostic
shape, including byte offset, field path, requested byte count, available byte
count, readiness, and bounded byte-preview details. Ordinary source type and
value positions continue to reject the primitive names with
`schema.exact_width_primitive`.

## Evidence

- `../../../examples/specification/run/binary-schema-u40-widths-encode/`
  checks five-byte big-endian and little-endian encode output and generated
  decode helper `Int` values.
- `../../../examples/specification/run/binary-schema-u40-widths-encode-out-of-range/`
  checks exact-width encode range failures.
- `../../../examples/specification/run/binary-schema-u40-widths-truncated-json/`
  checks the JSON truncation diagnostic and byte preview.
- `../../../examples/specification/check/schema-exact-width-primitive-diagnostics/`
  checks that `UInt40be` and `UInt40le` remain schema-local names.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
primitive widths, dispatch forms, and mapping behavior outside the implemented
generated-helper slices.
