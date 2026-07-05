# Binary Data Schema Conversion Boundary

Status: implemented

This record preserves the completed schema-facing byte conversion boundary
slice from the binary data standard-library proposal. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/names-effects-full.md`, and the checked executable cases
under `../../../examples/specification/run/`.

## Completed Behavior

Schema-facing byte data uses the existing immutable byte vocabulary without
adding implicit schema coercions or memory-layout guarantees.

- `byte_view(chunk, offset, count)` is the source-visible boundary from an
  owned `ByteChunk` to a bounded `ByteView` for schema input or payload fields.
- `byte_view_to_chunk(view)` is the source-visible boundary from a bounded
  schema-decoded `ByteView` back to owned `ByteChunk` data.
- Failed requested ranges remain ordinary byte-range failures. JSON diagnostics
  report `codec.byte_range_out_of_bounds`, the requested byte offset, requested
  count, available count, and a bounded byte preview outside the primary
  message.

The public model stays immutable and does not promise zero-copy behavior,
production memory layout, socket behavior, streaming behavior, or HPACK
behavior.

## Evidence

- `../../../examples/specification/run/binary-schema-byte-conversion-boundary/`
  checks successful `ByteChunk` to `ByteView` conversion for schema decode,
  schema-decoded `ByteView` materialization back to `ByteChunk`, and explicit
  `ByteChunk` to `ByteView` conversion before schema encode.
- `../../../examples/specification/run/binary-schema-byte-conversion-range-json/`
  checks the failed schema payload view conversion path and the structured
  byte-range diagnostic fields.
- `../../specification/names-effects.md` and
  `../../specification/names-effects-full.md` summarize the source-visible
  helper boundary.
