# Codec Generated Helper Boundary Slices

Status: implemented

This record preserves completed generated-helper-backed codec execution slices
from the superseded codec execution design. Source-level `codec`
declarations are no longer current source syntax; current schema operation
behavior is specified by `../../specification/execution.md`,
`../../specification/source-surface.md`, `../../specification/examples.md`,
and the checked executable examples under `../../../examples/specification/`.

## Outcome

Derived codec decode calls expose the same source-call boundary as the
generated `byte_decode_step_<schema>` helper when the named schema is already
eligible for that helper. The completed slices cover byte-aligned
representation-only `ReservedBits(width, value)` fields, addition,
subtraction, multiplication, and division repeated primitive count
expressions, standalone visible `UInt1` through `UInt7` fields, opt-in
visible flag bitset fields, including generated-helper-backed `Flag24be` and
`Flag24le` fields,
visible-only packed three-byte, four-byte, five-byte, six-byte, seven-byte,
and eight-byte groups, seven-byte
or eight-byte reserved prefix groups, seven-byte wide reserved suffix groups,
the general padded `ReservedBits(width, value)` plus `UInt8` prefix route,
and schema
mappings that call pure same-module or imported public converters with one or
more supported structural arguments. A codec
call receives a bounded `ByteView` and explicit base `ByteOffset`, returns
`Decoded` with the helper value and consumed `ByteCount`, returns `NeedMore`
without consuming input, and returns helper `Invalid(DecodeError)` values
without advancing caller-owned parser state. Helper-projected invalid input
reports the absolute byte offset from the explicit base offset and
field-local byte position, independent of the `ByteView` storage offset in
its source chunk.

Derived codec encode calls expose the same source-call boundary as the
generated `byte_encode_<schema>` helper when the named schema is already
eligible for that helper. The completed slices cover addition, subtraction,
multiplication, and division repeated primitive count expressions,
quotient-sized `ByteView(left_length / right_length)` payload fields,
standalone visible `UInt1` through `UInt7` fields, opt-in visible flag bitset
fields, including generated-helper-backed `Flag24be` and `Flag24le` fields,
visible-only packed three-byte, four-byte, five-byte, six-byte, seven-byte,
and eight-byte groups, seven-byte
or eight-byte reserved prefix groups, and seven-byte wide reserved suffix
groups, plus the general padded `ReservedBits(width, value)` plus `UInt8`
prefix route.
A codec call receives the helper value record, returns helper success as
`Encoded(List<ByteChunk>)`, and projects helper representation failures to
`Invalid(EncodeError)` before any hidden mutable output state exists. The
budgeted helper-backed path can expose `Partial` with emitted chunks,
produced count, and a resumable state record carrying `encoded_offset`.

## Evidence

- `../../../examples/specification/run/derived-codec-byte-aligned-reserved-decode-boundary/`
  checks generated helper decode behavior for byte-aligned
  representation-only `ReservedBits(width, value)` fields through the derived
  codec item, including successful `Decoded`, consumed count, short-input
  readiness, and helper `Invalid(DecodeError)` projection. The companion JSON
  case checks command-facing diagnostics for the helper-projected
  reserved-bit mismatch.
- `../../../examples/specification/run/derived-codec-middle-reserved-decode-boundary/`
  checks generated helper decode behavior for a middle reserved-bit layout
  through the derived codec item from nonzero bounded view offsets, including
  successful `Decoded`, non-consuming short-input readiness, and helper
  `Invalid(DecodeError)` projection at the explicit absolute base offset.
- `../../../examples/specification/run/derived-codec-flag-boundary/` checks
  successful flag-bitset decode, consumed count, short-input readiness,
  successful encode, output chunk projection, and helper encode failure
  projection through the derived codec item.
- `../../../examples/specification/run/derived-codec-byteview-quotient-encode-boundary/`
  checks quotient-sized `ByteView` encode success, length-mismatch helper
  failure projection, and division-by-zero helper failure projection through
  the derived codec item.
- `../../../examples/specification/run/derived-codec-repeat-arithmetic-boundary/`
  checks addition, subtraction, and multiplication repeat count helper
  decode and encode success, short-input readiness, helper decode failure
  projection, output chunk projection, and encode count-mismatch projection
  through the derived codec item.
- `../../../examples/specification/run/derived-codec-repeat-quotient-boundary/`
  checks division repeat count helper decode and encode success,
  short-input readiness, division-by-zero helper failure projection, output
  chunk projection, and encode count-mismatch projection through the derived
  codec item.
- `../../../examples/specification/run/derived-codec-wide-reserved-prefix-boundary/`
  checks seven-byte and eight-byte reserved prefix group decode through the
  derived codec item, non-consuming reserved-bit mismatch `Invalid` values,
  consumed counts, successful encode, and output chunk projection.
- `../../../examples/specification/run/binary-schema-reserved-nine-bit-prefix-decode-encode/`
  checks the `ReservedBits(9, 0)` plus `UInt8` padded prefix helper
  through the derived codec item, including successful `Decoded`, consumed
  count, short-input readiness, non-consuming reserved-bit mismatch
  `Invalid`, successful encode, output chunk projection, and helper encode
  failure projection.
- `../../../examples/specification/run/binary-schema-reserved-nine-bit-prefix-codec-json/`
  checks command-facing JSON projection for the helper-projected reserved-bit
  mismatch `Invalid(DecodeError)` value returned through the derived codec
  item.
- `../../../examples/specification/run/binary-schema-wide-suffix-reserved-seven-byte-decode-encode/`
  checks seven-byte wide reserved suffix decode through the derived codec item,
  short-input readiness, non-consuming reserved-bit mismatch `Invalid`, consumed
  count, successful encode, output chunk projection, and helper-projected
  encode failure.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
  checks helper availability diagnostics for schema helper shapes that remain
  unsupported.

## Remaining Work

The source-level codec route is closed by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md). Do not
extend this record by adding another same-shaped helper-backed layout; current
schema work should use explicit schema operations and ordinary functions.
