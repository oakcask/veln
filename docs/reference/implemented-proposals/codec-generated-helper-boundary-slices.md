# Codec Generated Helper Boundary Slices

Status: implemented

This record preserves completed generated-helper-backed codec execution slices
from `../../proposals/codec-execution-boundary.md`. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/source-surface.md`, `../../specification/examples.md`,
and the checked executable examples under `../../../examples/specification/`.

## Outcome

Derived codec decode calls expose the same source-call boundary as the
generated `byte_decode_step_<schema>` helper when the named schema is already
eligible for that helper. The completed slices cover addition, subtraction,
multiplication, and division repeated primitive count expressions, standalone
visible `UInt1` through `UInt7` fields, opt-in visible flag bitset fields,
visible-only packed three-byte and four-byte groups, seven-byte or eight-byte
reserved prefix groups, seven-byte wide reserved suffix groups, and schema
mappings that call pure same-module converters with five structural
arguments. A codec
call receives a bounded `ByteView` and explicit base `ByteOffset`, returns
`Decoded` with the helper value and consumed `ByteCount`, returns `NeedMore`
without consuming input, and returns helper `Invalid(DecodeError)` values
without advancing caller-owned parser state.

Derived codec encode calls expose the same source-call boundary as the
generated `byte_encode_<schema>` helper when the named schema is already
eligible for that helper. The completed slices cover addition, subtraction,
multiplication, and division repeated primitive count expressions,
quotient-sized `ByteView(left_length / right_length)` payload fields,
standalone visible `UInt1` through `UInt7` fields, opt-in visible flag bitset
fields, visible-only packed three-byte and four-byte groups, seven-byte or
eight-byte reserved prefix groups, and seven-byte wide reserved suffix groups. A codec
call receives the helper value record, returns helper success as
`Encoded(List<ByteChunk>)`, and projects helper representation failures to
`Invalid(EncodeError)` before any hidden mutable output state exists. The
budgeted helper-backed path can expose `Partial` with emitted chunks,
produced count, and a resumable state record carrying `encoded_offset`.

## Evidence

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
- `../../../examples/specification/run/derived-codec-sub-byte-boundary/`
  checks standalone visible `UInt1` through `UInt7` helper decode and encode
  success, short-input readiness, field-validation helper failure projection,
  helper encode failure projection, and budgeted partial/resume behavior
  through the derived codec item.
- `../../../examples/specification/run/derived-codec-packed-visible-three-byte-boundary/`
  checks visible-only packed three-byte helper decode and encode behavior
  through the derived codec item, including short-input readiness, budgeted
  partial/resume behavior, and helper encode failure projection.
- `../../../examples/specification/run/derived-codec-packed-visible-four-byte-boundary/`
  checks visible-only packed four-byte helper decode and encode behavior
  through the derived codec item, including short-input readiness, budgeted
  partial/resume behavior, and helper encode failure projection.
- `../../../examples/specification/run/derived-codec-five-argument-mapped-converter-decode-boundary/`
  checks generated helper decode behavior for a schema mapping that calls a
  pure same-module converter with five structural arguments through the
  derived codec item, including successful decode, consumed count, short-input
  readiness, and helper decode failure projection. The companion human and
  JSON cases check the command-facing diagnostics for that helper-projected
  `Invalid(DecodeError)` value.
- `../../../examples/specification/run/derived-codec-wide-reserved-prefix-boundary/`
  checks seven-byte and eight-byte reserved prefix group decode through the
  derived codec item, non-consuming reserved-bit mismatch `Invalid` values,
  consumed counts, successful encode, and output chunk projection.
- `../../../examples/specification/run/binary-schema-wide-suffix-reserved-seven-byte-decode-encode/`
  checks seven-byte wide reserved suffix decode through the derived codec item,
  short-input readiness, non-consuming reserved-bit mismatch `Invalid`, consumed
  count, successful encode, output chunk projection, and helper-projected
  encode failure.
- `../../../examples/specification/check/derived-codec-wide-suffix-helper-eligibility-diagnostics/`
  checks that an unsupported wide reserved suffix shape still rejects
  `derive decode` and `derive encode` when the matching generated helpers are
  unavailable.

## Remaining Work

The broader codec execution boundary proposal remains open for schema-driven
codec execution beyond the helper-backed slices already accepted by generated
binary schema helpers. Extending this record by adding another same-shaped
helper-backed layout is not a goal on its own; future work should either name
a specific language capability that still lacks a codec boundary or define a
more general codec abstraction.
