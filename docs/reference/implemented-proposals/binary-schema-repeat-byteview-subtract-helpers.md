# Binary Schema Repeat ByteView Subtract Helpers

Status: implemented

This record preserves the completed bounded repeat
`ByteView(left_length - right_length)` payload helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Generated binary schema decode and encode helpers accept repeated
`ByteView(left_length - right_length)` payloads when both operands name earlier
visible `Int` fields in the same schema. The repeated field exposes
`List<ByteView>` and uses the same count-expression boundary as other bounded
repeat fields.

Decode reads each element using the evaluated subtractive payload length.
Truncation inside a repeated `ByteView` payload reports
`schema.truncated_field` with the schema field path and repeated element index.

`ByteView(left_length * right_length)` and
`ByteView(left_length / right_length)` repeat payloads remain outside this
helper slice.

## Evidence

- `../../../examples/specification/run/binary-schema-repeat-byteview-subtract-decode/`
  checks direct generated helper decode over subtractive repeated `ByteView`
  payloads.
- `../../../examples/specification/run/binary-schema-repeat-byteview-subtract-truncated-json/`
  checks the repeated element path and byte-count facts for truncation.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation and IR metadata.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, format-neutral encode helper fields
beyond the implemented boundary, and later schema composition surfaces.
