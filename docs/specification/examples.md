# Implemented Examples

Status: implemented

This file records complete examples that are expected to type-check and run
against the implemented language specification.

## Line-Item Order Summary

The comparison example is implemented in `examples/comparison/`. Its rationale
is recorded in
[Comparison Example Task](../reference/source-decisions/records/result-comparison-example-task.md).

The pure API is:

```veln
summarize_order(lines: Vec<String>, catalog: Dict<String, Int>) -> Result<{item_count: Int, subtotal_cents: Int}, {kind: String, input: String}>
```

Input lines use `sku,quantity` spelling. The implementation rejects malformed
rows, non-integer or non-positive quantities, and unknown SKUs. The command
wrapper keeps stdout in `main` and leaves parsing and summarization in pure
functions.

The example uses these implemented language features together:

- dictionary lookup with `dict_get`
- fallible vector traversal with `vec_try_map_with`
- summary accumulation with `vec_fold`
- `Result` propagation
- record-shaped success and error values
- `stdio::println` for the wrapper
- a separate partial-program variant with a constrained typed hole
- canonical `#` source comments on example-authored notes

## Variadic Functions

The executable specification case
`../../examples/specification/check/variadic-function-arguments/` covers
source-level variadic declaration parameters, variadic function types, ordinary
calls, pipeline calls, and function-value assignment compatibility.

The sibling check cases
`../../examples/specification/check/variadic-function-diagnostics/`,
`../../examples/specification/check/variadic-call-diagnostics/`,
`../../examples/specification/check/variadic-marker-type-boundaries/`, and
`../../examples/specification/check/variadic-spread-call-boundary/` pin
placement diagnostics, missing element types, ordinary type-position marker
rejection, missing fixed arguments, wrong variadic tail element types,
variadic/fixed callable mismatch, and the current no-spread-call boundary.

The executable run cases
`../../examples/specification/run/variadic-entry-arguments/` and
`../../examples/specification/run/variadic-entry-argument-diagnostics/` cover
command-line conversion for fixed plus variadic entry arguments and rejection
of unsupported variadic entry element types.

## Binary Fixture Records

The executable specification case
`../../examples/specification/run/binary-fixture-records/` keeps named valid
and invalid binary fixtures inside the example tree. The fixture records carry
the fixture name, decoded `ByteChunk`, optional consumed `ByteCount`, and
expected invalid-fixture error text without adding production standard-library
API beyond `byte_chunk_from_hex`.

The toolchain harness checks each named fixture through complete lowercase hex
in `case.toml`, plus decoded byte count and optional consumed count. Valid
fixture records keep the source-owned `ByteChunk` separate from the lowercase
hex expectation used for machine comparison. Invalid fixture records are
checked by their stable error text. This is executable specification evidence
for fixture ownership and expected-output comparison, not a public
serialization surface.

The same case also pins protocol-facing expected output chunk lists. A
`[[output_chunk_list]]` manifest entry names an ordered list of complete
lowercase hex chunks. The harness compares the named list against consecutive
program-output lines that include the list count, each chunk index, each exact
hex string, and the decoded byte count. Empty chunk lists and zero-length
chunks are distinct and checked separately.

`../../examples/specification/run/binary-fixture-truncated-input-json/` shows a
named fixture record whose valid decoded bytes are intentionally too short for
the read under test. The case metadata keeps the fixture name, complete
lowercase hex, decoded byte count, expected consumed count, byte offset,
expected byte count, available byte count, readiness, and empty direct-read
field path separate from the `codec.incomplete_input` JSON assertion.
`../../examples/specification/run/binary-fixture-invalid-field/` shows a named
fixture record with schema-aware metadata for a same-module schema reference
and a matching structured field path.
`../../examples/specification/run/binary-fixture-schema-references/` checks
same-module, imported public schema, and imported public schema-alias fixture
references with matching structured field paths. The companion
`../../examples/specification/run/binary-fixture-schema-reference-diagnostics/`
case pins manifest-time rejection for missing, private imported, wrong-kind,
generated-helper, missing-use, and field-path-mismatched schema references.

## Binary Byte Views

The executable specification case
`../../examples/specification/run/binary-byteview/` covers source-visible
`ByteView` slices, checked unsigned big-endian and little-endian reads,
checked unsigned big-endian and little-endian writes, truncation failures,
range failures, and conversion overflow failures without relying on HTTP/2 or
codec declarations. It also passes a `ByteView` through a channel and reads
the received view, then materializes the received view as `ByteChunk`, to
cover the ordinary immutable freeze boundary.

The sibling failure cases under `../../examples/specification/run/` pin the
runtime `Result` propagation shape for ByteView read truncation, ByteView range
failure, and unsigned write conversion overflow in JSON and human command
output. The read-truncation JSON and human cases pin the
`codec.incomplete_input` byte diagnostic details and missing-byte human
projection. The range JSON and human cases pin
`codec.byte_range_out_of_bounds` byte diagnostic details, including requested
count, available count, and bounded nearby-byte preview context. The write
overflow JSON and human cases pin
`codec.byte_write_value_unrepresentable` value diagnostic details, including
helper name, supplied value, accepted range, width, and byte order. The
named-fixture truncation case pins the same JSON diagnostic shape while
proving that valid fixture bytes fail as codec truncation, not as fixture text
validation.
The `binary-byteview-u40-u48-helpers` case covers source-visible five-byte and
six-byte helper reads and writes in both byte orders, including short input and
unrepresentable write failures. The `binary-byteview-u48-write-failure-json`
case pins the structured value diagnostic details for a six-byte helper.

## Codec Decode Step Vocabulary

The executable specification case
`../../examples/specification/run/codec-decode-step-vocabulary/` covers the
source-visible incremental decode transition vocabulary. Ordinary source
functions construct `DecodeStep<T>` values for a successful `Decoded` outcome
with a decoded value and consumed `ByteCount`, a `NeedMore` outcome with
`NeedBytes` readiness that consumes no input, and an `Invalid` outcome carrying
the base `DecodeError(...)` constructor with id, byte offset, and field path.
The executable specification case
`../../examples/specification/run/binary-schema-decode-step/` covers the
generated schema-derived decode-step helper: complete buffered input returns
`Decoded` with the exact consumed count, and short open input returns
`NeedMore(NeedBytes(...))` without consuming bytes.
The executable specification case
`../../examples/specification/run/codec-decode-boundary/` covers a
hand-written codec decode boundary: a codec item call passes `ByteView` and
`ByteOffset` to the referenced decoder, observes valid `Decoded`,
`NeedMore`, and `Invalid` `DecodeStep<T>` values, and projects an oversized
consumed count to `codec.consumed_count_invalid` while the schema mapping pins
the accepted value type.
The executable specification cases
`../../examples/specification/run/codec-decode-consumed-count-invalid-human/`
and
`../../examples/specification/run/codec-decode-consumed-count-invalid-json/`
cover command-facing projection of the hand-written codec boundary's stable
`codec.consumed_count_invalid` decode failure without treating it as retryable
readiness.
The executable specification case
`../../examples/specification/run/codec-decode-decoded-json/` covers the
successful `Decoded(...)` entry boundary in `run --json`: a decoded entry
returns ordinary stdout, `status = passed`, and `error = null` without a byte
diagnostic failure projection.
The executable specification cases
`../../examples/specification/run/codec-decode-invalid-boundary-human/` and
`../../examples/specification/run/codec-decode-invalid-boundary-json/` cover
command-facing projection when a hand-written `decode with` codec boundary
returns a codec-owned reason-carrying `Invalid(DecodeErrorWithReason(...))`:
human output reports the failed decode fact at the contained byte offset with
related field-path, reason, and source-visible value notes, and `run --json`
attaches `details.byte_diagnostic.reason`.
The executable specification cases
`../../examples/specification/run/codec-decode-invalid-byte-context-human/`
and
`../../examples/specification/run/codec-decode-invalid-byte-context-json/`
cover the same hand-written codec boundary when the reason is a byte-helper
failure message with registered byte context: human output adds local byte
offset, expected and available byte counts, and bounded nearby bytes as
related notes, and `run --json` attaches the same fields under
`details.byte_diagnostic`.
The executable specification cases
`../../examples/specification/run/codec-decode-invalid-step-human/` and
`../../examples/specification/run/codec-decode-invalid-step-json/` cover
command-facing projection when a `veln run` entry returns
`Invalid(DecodeError(...))`: human output reports the failed decode fact at
the contained byte offset with related field-path and source-visible value
notes, and `run --json` attaches `details.byte_diagnostic`.
The executable specification cases
`../../examples/specification/run/codec-decode-need-more-human/` and
`../../examples/specification/run/codec-decode-need-more-json/` cover
command-facing projection when a `veln run` entry returns
`NeedMore(NeedBytes(...))` at a closed-input reporting boundary: human output
reports `codec.incomplete_input` with readiness and requested-byte context in
related notes, and `run --json` attaches `details.byte_diagnostic`.
The executable specification cases
`../../examples/specification/run/codec-decode-need-end-human/` and
`../../examples/specification/run/codec-decode-need-end-json/` cover the
same command-facing projection for `NeedMore(NeedEnd)` without requested-byte
context.
The executable specification case
`../../examples/specification/run/derived-codec-decode-boundary/` covers a
derived codec decode boundary for the same eligible generated binary schema
decode-step slice: a codec item call observes the generated helper's
`Decoded`, `NeedMore`, and `Invalid` `DecodeStep<T>` values through the codec
item name while preserving mapped record fields and no-consumption outcomes.
The executable specification case
`../../examples/specification/run/derived-codec-middle-reserved-decode-boundary/`
covers the same derived codec call boundary for a middle reserved-bit binary
layout. It checks successful decode, short-input readiness, and a
reserved-bit mismatch whose `DecodeError` preserves the reserved field path
and byte offset.
The executable specification case
`../../examples/specification/run/codec-needmore-parser-state/` covers
caller-owned parser state around the codec boundary. It checks that `Decoded`
advances the retained suffix and explicit base offset by the consumed count,
then decodes over that suffix, while `NeedMore` keeps the same pending bytes
and base offset.
The executable specification case
`../../examples/specification/run/derived-codec-repeat-decode-boundary/`
covers the same derived codec call boundary when the generated decode-step
helper decodes a bounded repeated primitive field and reports repeat-backed
readiness or helper failure through the codec item.
The executable specification case
`../../examples/specification/run/derived-codec-nested-dispatch-decode-boundary/`
covers the same derived codec call boundary when the generated decode-step
helper decodes a same-module nested dispatch payload schema whose generated
helper uses field-local validation, reserved fields, and little-endian reads.
The executable specification case
`../../examples/specification/run/derived-codec-recursive-dispatch-boundary/`
covers the same derived codec call boundary when the generated decode-step
helper decodes same-module recursive closed and extension dispatch payloads.
It checks recursive success, short-input `NeedMore`, helper failure
`Invalid`, and extension unknown-payload preservation through the codec item.
The executable specification case
`../../examples/specification/run/binary-schema-general-helper-roundtrip/`
covers the same derived codec decode boundary over one non-HTTP schema that
combines `Flag8`, bounded repeat fields, representation-only reserved fields,
`ByteView(left_length - right_length)`, same-module nested
`ExtensionDispatch` payloads, and little-endian nested primitive fields. It
checks successful decode, short-input `NeedMore(NeedBytes(...))`, and helper
failure projection to `Invalid(DecodeError)`.
The executable specification case
`../../examples/specification/run/codec-selected-mapping-decode-boundary/`
covers codec item calls over a schema with multiple decoded-field selected
mappings that resolve to one target record shape, including both derived and
hand-written decode boundaries.
The executable specification case
`../../examples/specification/check/codec-selected-mapping-boundary-diagnostics/`
covers hand-written decode functions that return the raw schema-local record
or another wrong record shape instead of the selected mapping target shape.
The executable specification case
`../../examples/specification/check/derived-codec-mapping-boundary-diagnostics/`
covers mapped derived encode clauses whose generated helper boundary cannot
project the schema mapping target value back to schema-local fields.
The executable specification cases
`../../examples/specification/check/derived-codec-helper-eligibility-diagnostics/`
and
`../../examples/specification/check/derived-codec-helper-eligibility-human/`
cover unsupported derived codec helper directions and their related schema
context in JSON and human output.

## Codec Encode Step Vocabulary

The executable specification case
`../../examples/specification/run/codec-encode-step-vocabulary/` covers the
source-visible incremental encode transition vocabulary. Ordinary source
functions construct `EncodeStep<TState>` values for complete `Encoded`
`List<ByteChunk>` output, `Partial` committed chunks with produced
`ByteCount` and resumable state, and an `Invalid` outcome carrying a
structured `EncodeError` with id, field path, and representation-failure
reason.
The executable specification case
`../../examples/specification/run/codec-encode-boundary/` covers a
hand-written codec encode boundary: a codec item call passes the mapped record
value and ordinary encoder parameters to the referenced encoder and observes
its returned `Encoded`, `Partial`, and `Invalid(EncodeError)`
`EncodeStep<TState>` values unchanged. The partial path keeps the emitted
chunk list, produced `ByteCount`, and resumed encoder state source-visible,
then uses the returned state to complete a later encode call.
The executable specification cases
`../../examples/specification/run/codec-encode-invalid-step-human/` and
`../../examples/specification/run/codec-encode-invalid-step-json/` cover
command-facing projection when a hand-written codec encode entry returns
`Invalid(EncodeError(...))`: human output uses the focused encode diagnostic,
and `run --json` attaches `details.value_diagnostic`.
The executable specification case
`../../examples/specification/run/derived-codec-encode-boundary/` covers a
derived codec encode boundary for the eligible generated binary schema encode
helper slice: a codec item call observes successful helper output as
`Encoded(List<ByteChunk>)` with one chunk and out-of-range generated helper
failures as `Invalid(EncodeError)`.
The executable specification case
`../../examples/specification/run/derived-codec-budgeted-encode-boundary/`
covers the budgeted form of that boundary: a codec item call accepts the same
value record plus a `ByteCount` output budget, returns complete output as
`Encoded`, returns oversized output as `Partial` with the committed prefix and
a state record carrying `encoded_offset`, resumes by passing that state record
back to the codec with a later budget, and still projects helper failures to
`Invalid` before exposing output.
The executable specification case
`../../examples/specification/run/derived-codec-mapped-encode-boundary/`
covers the same boundary when a direct structural mapping makes the generated
helper accept the mapping target record shape.
The executable specification case
`../../examples/specification/run/derived-codec-record-payload-mapped-encode-boundary/`
covers the same boundary when that mapped target record contains an ADT
constructor field whose payload is a record projected back to schema-local
fields.
The executable specification case
`../../examples/specification/run/derived-codec-repeat-encode-boundary/`
covers the same derived codec call boundary when the generated encode helper
writes a bounded repeated primitive field.
The executable specification case
`../../examples/specification/run/derived-codec-nested-dispatch-encode-boundary/`
covers the same derived codec call boundary when the generated encode helper
writes a same-module nested dispatch payload schema whose generated helper
uses reserved fields and little-endian output, and projects dispatch selection
failures as `Invalid(EncodeError)`.
The executable specification case
`../../examples/specification/run/derived-codec-recursive-dispatch-boundary/`
covers the same derived codec call boundary when the generated encode helper
writes same-module recursive closed and extension dispatch payloads through
the codec item and projects successful helper output to one encoded chunk.
The executable specification case
`../../examples/specification/run/binary-schema-general-helper-roundtrip/`
covers the same derived codec encode boundary over the combined non-HTTP
schema shape listed above and checks that helper `Ok(ByteChunk)` output
projects to one `Encoded(List<ByteChunk>)` chunk, while helper
`Err(EncodeError)` output projects to `Invalid(EncodeError)`.
The derived mapping-boundary diagnostics case listed above pins the matching
`codec.derive_helper_unsupported` rejection for generated encode boundaries.

## Binary Schema Frame Header

The executable specification cases
`../../examples/specification/run/binary-schema-width-sample-decode/` and
`../../examples/specification/run/binary-schema-width-sample-truncated-json/`
cover the implemented `UInt16be` and `UInt32be` primitive decode slice. The
valid case checks both fields over one `ByteView` and observes ordinary `Int`
record fields. The failure case pins `schema.truncated_field` details for a
truncated `UInt32be` field, including byte offset, schema field path, expected
byte count, available byte count, readiness, and structured byte preview
fields.

`../../examples/specification/check/binary-schema-u16le/`,
`../../examples/specification/run/binary-schema-u16le-decode/`,
`../../examples/specification/run/binary-schema-u16le-encode/`, and
`../../examples/specification/run/binary-schema-u16le-encode-out-of-range/`
cover the implemented `UInt16le` primitive slice. The source case also pins
accepted `UInt24le`, `UInt31le`, and `UInt32le` `format binary` field use.
`../../examples/specification/run/binary-schema-little-endian-widths-decode/`,
`../../examples/specification/run/binary-schema-little-endian-widths-encode/`,
and
`../../examples/specification/run/binary-schema-little-endian-widths-encode-out-of-range/`
cover the `UInt24le`, `UInt31le`, and `UInt32le` slice. The runtime cases prove
little-endian decode and encode byte order, preserve structural mapping during
decode, and pin generated encode helper range failures with maximum values
derived from each primitive width.
`../../examples/specification/run/binary-schema-u31le-integer-out-of-range-json/`
and
`../../examples/specification/run/binary-schema-u31le-integer-out-of-range-human/`
pin `schema.integer_out_of_range` for a structurally decoded `UInt31le` field
whose high bit exceeds the 31-bit external range.
`../../examples/specification/run/binary-schema-u48-widths-decode/`,
`../../examples/specification/run/binary-schema-u48-widths-encode/`, and
`../../examples/specification/run/binary-schema-u48-widths-encode-out-of-range/`
cover the `UInt48be` and `UInt48le` schema primitive slice for
source-visible `Int` values. The runtime cases prove big-endian and
little-endian byte order, structural mapping during decode, and generated
encode helper range failures at the unsigned 48-bit boundary.
`../../examples/specification/run/binary-schema-u56-widths-decode/`,
`../../examples/specification/run/binary-schema-u56-widths-encode/`,
`../../examples/specification/run/binary-schema-u56-widths-truncated-json/`,
and
`../../examples/specification/run/binary-schema-u56-widths-encode-out-of-range/`
cover the `UInt56be` and `UInt56le` schema primitive slice for
source-visible `Int` values. The runtime cases prove seven-byte big-endian
and little-endian byte order, structural mapping during decode, the shared
`schema.truncated_field` diagnostic shape, and generated encode helper range
failures at the unsigned 56-bit boundary.
`../../examples/specification/run/binary-schema-u64-widths-decode/`,
`../../examples/specification/run/binary-schema-u64-widths-encode/`,
`../../examples/specification/run/binary-schema-u64-widths-truncated-json/`,
and
`../../examples/specification/run/binary-schema-u64-widths-encode-out-of-range/`
cover the `UInt64be` and `UInt64le` schema primitive slice for
source-visible `Int` values. The runtime cases prove big-endian and
little-endian byte order, the shared `schema.truncated_field` diagnostic
shape, and generated encode helper range failures.
`../../examples/specification/run/binary-byteview-u64-helpers/`,
`../../examples/specification/run/binary-byteview-u40-u48-helpers/`,
`../../examples/specification/run/binary-byteview-u64-truncated-json/`,
`../../examples/specification/run/binary-byteview-u64-write-failure-human/`,
`../../examples/specification/run/binary-byteview-u64-write-failure-json/`,
and
`../../examples/specification/run/binary-byteview-u48-write-failure-json/`
cover the ordinary prelude byte-helper `u40`, `u48`, and `u64` slices. The
runtime cases prove big-endian and little-endian read byte order, matching
write byte order, truncated-read diagnostics, and the source-visible `Int`
write boundary including little-endian write diagnostic details.

`../../examples/specification/run/binary-schema-integer-out-of-range-json/`
and
`../../examples/specification/run/binary-schema-integer-out-of-range-human/`
pin `schema.integer_out_of_range` for a structurally decoded `UInt31be` field
whose raw integer exceeds that primitive's external range. The cases assert
byte offset, schema field path, byte width, accepted range, actual value, and
structured or rendered byte preview fields.

The executable specification cases
`../../examples/specification/run/binary-schema-frame-header-decode/`,
`../../examples/specification/run/binary-schema-frame-header-truncated-json/`,
and
`../../examples/specification/run/binary-schema-frame-header-reserved-json/`
cover the generated `Http2FrameHeaderWire` helper path. The valid case checks
`UInt24be`, two `UInt8` fields, `ReservedBits(1, 0)`, and `UInt31be` over one
`ByteView`, and its expected record omits the reserved field. The failure
cases pin structured `schema.truncated_field` and
`schema.reserved_bits_mismatch` details, including byte offsets and schema
field paths. The sibling human-output cases
`../../examples/specification/run/binary-schema-frame-header-truncated-human/`
and
`../../examples/specification/run/binary-schema-frame-header-reserved-human/`
pin the focused primary messages and related notes for readiness, expected
versus available bytes, reserved-bit values, nearby bytes, and schema field
paths.

`../../examples/specification/run/binary-schema-byte-aligned-reserved-decode/`,
`../../examples/specification/run/binary-schema-byte-aligned-reserved-json/`,
and
`../../examples/specification/run/binary-schema-byte-aligned-reserved-truncated-json/`
pin byte-aligned `ReservedBits(width, value)` decode. The valid case consumes
the reserved bytes without exposing the field in the decoded record. The
failing cases assert `schema.reserved_bits_mismatch` and
`schema.truncated_field` with the reserved field path, byte offset, expected
value or byte count, actual value or available count, and structured byte
preview fields.
`../../examples/specification/run/binary-schema-packed-reserved-decode/`,
`../../examples/specification/run/binary-schema-packed-reserved-json/`,
`../../examples/specification/run/binary-schema-packed-reserved-four-byte-decode/`,
`../../examples/specification/run/binary-schema-packed-reserved-four-byte-json/`,
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-json/`,
`../../examples/specification/run/binary-schema-packed-reserved-three-byte-decode/`,
and
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-truncated-json/`
pin the packed reserved-bit decode slice. The valid case decodes high
`ReservedBits(width, value)` prefixes for widths one through seven plus the
visible `UIntN` field that completes each byte and widths nine through
fifteen plus the visible `UIntN` field that completes each two-byte
big-endian storage unit. The three-byte and four-byte cases decode high
reserved prefixes plus the visible `UIntN` field that completes the storage
unit. The reserved field is omitted from the decoded
record, and the helper then reads the following field after the shared storage
unit. The failing cases assert `schema.reserved_bits_mismatch` and
`schema.truncated_field` for the packed reserved field. The checked
diagnostics case
`../../examples/specification/check/schema-packed-reserved-mapping-diagnostics/`
asserts that the packed reserved field is not available as a structural
mapping source field.
`../../examples/specification/run/binary-schema-packed-reserved-suffix-decode/`,
`../../examples/specification/run/binary-schema-packed-reserved-suffix-json/`,
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-decode/`,
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-json/`,
`../../examples/specification/run/binary-schema-packed-reserved-three-byte-decode/`,
`../../examples/specification/run/binary-schema-packed-reserved-four-byte-decode/`,
`../../examples/specification/run/binary-schema-packed-reserved-three-byte-suffix-json/`,
`../../examples/specification/run/binary-schema-packed-reserved-suffix-truncated-json/`,
and
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-truncated-json/`
pin the packed reserved suffix slice. The valid cases decode the visible high
bits and omit the low reserved suffix field from the decoded record for
one-byte, two-byte, three-byte, and four-byte shared storage units. The
failing cases
assert `schema.reserved_bits_mismatch` at the reserved suffix field path and
`schema.truncated_field` at the visible field path when the shared storage
unit is missing or incomplete.
`../../examples/specification/run/binary-schema-middle-reserved-decode-encode/`
and
`../../examples/specification/run/binary-schema-middle-reserved-json/` pin the
middle reserved-bit slice. The valid case decodes adjacent visible fields
around a middle reserved field and omits the reserved field from the decoded
record. The failing case asserts `schema.reserved_bits_mismatch` at the
middle reserved field path with stable byte offset and bit-value details.
`../../examples/specification/run/binary-schema-byte-interleaved-middle-reserved-decode-encode/`
and
`../../examples/specification/run/binary-schema-byte-interleaved-middle-reserved-json/`
pin the narrow byte-interleaved middle reserved-bit slice where a visible
sub-byte field, a reserved field, a visible `UInt8`, and a final visible
sub-byte field share one two-byte storage unit.

The executable specification cases
`../../examples/specification/run/binary-schema-frame-payload-decode/`,
`../../examples/specification/run/binary-schema-frame-payload-length-json/`,
and
`../../examples/specification/run/binary-schema-frame-payload-length-human/`
cover the bounded HTTP/2 frame payload slice. The valid case observes payload
bytes through the returned `payload: ByteView` separately from the header
fields. The failure cases pin `schema.length_out_of_bounds` for a complete
header whose decoded length exceeds the available payload bytes, including the
first missing byte offset, expected and available counts, structured byte
preview fields in JSON, human nearby-byte notes, and `Http2FrameHeader.payload`
field path.

`../../examples/specification/run/binary-fixed-field-mismatch-json/`,
`../../examples/specification/run/binary-fixed-field-mismatch-human/`,
`../../examples/specification/run/binary-schema-fixed-field-mismatch-json/`,
and
`../../examples/specification/run/binary-schema-fixed-field-mismatch-human/`
pin schema-owned fixed-field mismatch diagnostics through direct byte helpers
and generated binary schema decode helpers. The JSON cases assert
`schema.fixed_field_mismatch`, decoded byte offset, structured field path,
expected and actual values, and structured byte preview fields. The human
cases assert that the primary message stays focused on the fixed-field
mismatch and puts field path, expected value, actual value, and nearby context
in related notes.

`../../examples/specification/run/binary-schema-validation-decode/`,
`../../examples/specification/run/binary-schema-validation-json/`, and
`../../examples/specification/run/binary-schema-validation-human/` pin the
first field-local schema `where` validation slice. The passing case preserves
the decoded record shape. The failing cases assert `schema.validation_failed`,
the owning field byte offset, structured field path, failed predicate text,
decoded values, structured byte preview fields, and the focused human primary
message.
`../../examples/specification/check/schema-declaration-diagnostics/` pins the
matching declaration boundary: malformed field-local `where` syntax remains a
`check` parse diagnostic instead of becoming a runtime schema validation
failure. `../../examples/specification/check/schema-declaration-human/` pins
the same boundary through human `check` output.

`../../examples/specification/run/binary-schema-validation-arithmetic-decode/`
and `../../examples/specification/run/binary-schema-validation-arithmetic-json/`
pin generated `byte_decode_<schema>` helpers for another schema declaration.
The passing case decodes an exact-width arithmetic predicate. The failing case
asserts the same `schema.validation_failed` shape with decoded values keyed by
schema field name.

`../../examples/specification/run/schema-value-validation/`,
`../../examples/specification/run/schema-value-validation-json/`, and
`../../examples/specification/run/schema-value-validation-human/` pin generated
`validate_<schema>` helpers for ordinary supplied decoded records. The passing
case returns the supplied record after field-local validation succeeds. The
failing cases assert the value-diagnostic `schema.validation_failed` shape,
schema and field path, predicate text, owning supplied field value, supplied
decoded values, and the focused human primary message.

`../../examples/specification/run/binary-schema-structural-validation-decode/`,
`../../examples/specification/run/binary-schema-structural-validation-json/`,
and
`../../examples/specification/run/binary-schema-structural-validation-human/`
pin schema-level `validate` predicates after field decode and field-local
validation. The passing case preserves the decoded record shape. The failing
cases assert `schema.validation_failed`, the offset after the decoded schema
body, schema path, predicate text, decoded values, structured byte preview
fields, and the focused human primary message.

`../../examples/specification/run/binary-schema-mapped-record-decode/` pins
the generated schema mapping slice. The helper decodes exact-width schema
fields, checks the field-local predicate, and returns the mapped ordinary
record field names rather than the schema-local field names.
`../../examples/specification/run/binary-schema-mapped-record-expression-decode/`
and
`../../examples/specification/run/binary-schema-mapped-constructor-expression-decode/`
pin the structural mapping expression slice. The helper constructs nested
record and ADT target fields from decoded schema-local values.
`../../examples/specification/run/binary-schema-nested-mapped-constructor-decode/`
pins nested ADT constructor payload expressions in generated decode mapping:
the helper builds an outer constructor whose payload is another constructor
expression over decoded schema-local values.
`../../examples/specification/run/binary-schema-mapping-arithmetic-decode/`
pins the decoded-field and integer-literal arithmetic mapping slice. The helper
evaluates supported `+`, `-`, `*`, and `/` expressions after field-local
validation and returns the computed `Int` target fields. The
`../../examples/specification/run/binary-schema-mapping-converter-arithmetic-decode/`
and
`../../examples/specification/run/binary-schema-imported-mapping-converter-arithmetic-decode/`
cases pin same-module and imported public `Int` converter calls as supported
arithmetic operands. The
`../../examples/specification/run/binary-schema-mapping-division-by-zero-json/`
case pins the division-by-zero diagnostic shape.
`../../examples/specification/run/binary-schema-mapping-ordered-comparison-decode/`
pins ordered `Int` mapping comparisons into `Bool` target fields, including
composition with `and` and `not`, an `Int`-returning converter-call operand,
and nested integer arithmetic operands.
`../../examples/specification/run/binary-schema-mapped-converter-adt-argument-decode/`
and
`../../examples/specification/run/binary-schema-imported-mapped-converter-structural-argument-decode/`
pin converter arguments built from structural mapping expressions: a
same-module pure converter receives an ADT constructor expression, and an
imported public pure converter receives a record expression.
`../../examples/specification/run/binary-schema-two-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-two-argument-mapped-converter-decode/`
pin same-module and imported public two-argument converter calls.
`../../examples/specification/run/binary-schema-three-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-three-argument-mapped-converter-decode/`
pin same-module and imported public three-argument converter calls.
`../../examples/specification/run/binary-schema-four-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-four-argument-mapped-converter-decode/`
pin same-module and imported public four-argument converter calls.
`../../examples/specification/run/binary-schema-mapping-selection-decode/`
pins deterministic mapping selection by an already decoded field value.
`../../examples/specification/run/binary-schema-mapping-selection-not-equal-decode/`
pins inequality mapping selection by the same decoded field value.
`../../examples/specification/run/binary-schema-boolean-selected-mapping-decode/`
pins boolean mapping selection with `and`, `or`, and `not` over decoded
schema-local `Int` fields.
`../../examples/specification/run/binary-schema-mapping-ordered-selection-decode/`
pins selected mapping clauses that use ordered comparisons over decoded
schema-local `Int` fields and integer literals.
`../../examples/specification/run/binary-schema-mapped-field-selection-decode/`
pins mapping assignment field selection from a decoded nested record value.
`../../examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-decode/`
pins selected mapping branches that wrap mixed primitive and nested closed
dispatch payloads into one target record shape.
`../../examples/specification/check/schema-mapping-field-selection-diagnostics/`
pins missing selected fields and selection from non-record mapping values.
`../../examples/specification/check/schema-mapping-selection-diagnostics/`
pins JSON diagnostics for missing, duplicate, overlapping, and unsupported
mapping selection.
`../../examples/specification/check/schema-mapping-boolean-selector-diagnostics/`
pins JSON diagnostics for unsupported boolean selector expressions, unknown
selector fields, non-`Int` selector fields, and boolean-selector overlap.
`../../examples/specification/check/schema-mapping-ordered-comparison-diagnostics/`
pins JSON diagnostics for ordered comparison non-`Int` operands, non-`Bool`
target shapes, and unsupported ordered-comparison operand forms.
`../../examples/specification/check/schema-mapping-expression-boundary-diagnostics/`
pins unsupported mapping expression, unresolved constructor, constructor
arity, direct and nested constructor payload type, non-`Int` arithmetic
operand, and unsupported arithmetic expression diagnostics.
`../../examples/specification/check/schema-mapping-converter-arithmetic-diagnostics/`
and
`../../examples/specification/check/schema-mapping-converter-arithmetic-diagnostics-human/`
pin JSON and human diagnostics for converter resolution, arity, input, return,
purity, and unsupported converter arguments from arithmetic operands.

`../../examples/specification/run/binary-schema-primitive-encode/` and
`../../examples/specification/run/binary-schema-primitive-encode-out-of-range/`
pin the generated exact-width primitive encode helper slice. The passing case
encodes `UInt16be` followed by `UInt32be` into one immutable `ByteChunk` and
checks complete lowercase hex output. The failing case matches the returned
`EncodeError` and asserts `codec.encode_value_unrepresentable`, the schema
field path, and the `UInt31be` maximum.
`../../examples/specification/run/binary-schema-mapped-record-encode/` pins
the direct structural mapping encode helper slice: the helper accepts the
mapping target record shape, projects target fields back to schema-local
fields, and writes one immutable `ByteChunk`.
`../../examples/specification/run/derived-codec-selected-mapping-encode-boundary/`
pins the same generated helper behavior through a `derive encode` codec
boundary for selected structural mappings: both selector cases encode through
the mapped target record shape, and representation failures project to
`EncodeStep::Invalid`.
`../../examples/specification/run/binary-schema-mapped-record-expression-encode/`
pins the same inverse projection when one mapped target field is a record
value whose fields recover schema-local fields.
`../../examples/specification/run/binary-schema-mapped-field-selection-encode/`
pins the same inverse projection when a mapped target field selects a direct
schema-local field from a record-shaped mapping expression.
`../../examples/specification/run/binary-schema-imported-mapped-converter-encode/`
and
`../../examples/specification/run/binary-schema-imported-mapped-converter-encode-mismatch/`
pin converter inverse projection through imported public pure forward and
inverse converters written with import paths. The passing case writes the
recovered schema-local field, and the mismatch case reports
`codec.encode_mapping_mismatch` when the inverse projection does not
round-trip through the forward converter.
`../../examples/specification/run/derived-codec-imported-mapped-converter-encode-boundary/`
pins the same generated helper eligibility through a `derive encode` codec
boundary.
`../../examples/specification/run/binary-schema-int-mapped-constructor-encode/`
and
`../../examples/specification/run/binary-schema-int-mapped-constructor-encode-out-of-range/`
pin the direct single-constructor ADT inverse mapping slice for schema-local
exact-width integer fields. The passing case projects an integer payload back
to a `UInt16le` field and checks lowercase hex output. The
failing case preserves the ordinary `codec.encode_value_unrepresentable`
shape on the schema-local field path.
`../../examples/specification/run/binary-schema-multi-payload-mapped-constructor-encode/`
and
`../../examples/specification/run/binary-schema-multi-payload-mapped-constructor-encode-mismatch/`
pin direct multi-payload ADT inverse mapping. The passing case projects two
constructor payloads back to schema-local fields. The failing case reports
`codec.encode_mapping_mismatch` when the target field carries a different
constructor than the mapping expects.
`../../examples/specification/run/binary-schema-mapped-constructor-field-selection-encode/`
pins the same ADT inverse projection when a constructor payload is recovered
through field selection from a record-shaped mapping expression.
`../../examples/specification/run/binary-schema-record-payload-mapped-constructor-encode/`,
`../../examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-mismatch/`,
`../../examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-mismatch-json/`,
and
`../../examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-out-of-range/`
pin the record-payload ADT inverse mapping slice. The passing case
destructures the expected constructor and its record payload, then projects
record fields back to schema-local fields. The mismatch case reports
`codec.encode_mapping_mismatch` when the target field carries a different
constructor; the JSON variant pins that id in run result diagnostic details.
The range case preserves the ordinary
`codec.encode_value_unrepresentable` shape on the projected schema-local
field path.
`../../examples/specification/run/binary-schema-nested-mapped-constructor-encode/`,
`../../examples/specification/run/binary-schema-nested-mapped-constructor-encode-outer-mismatch-json/`,
`../../examples/specification/run/binary-schema-nested-mapped-constructor-encode-inner-mismatch-json/`,
and
`../../examples/specification/run/binary-schema-nested-mapped-constructor-encode-out-of-range/`
pin nested ADT constructor inverse mapping. The passing case projects the
inner constructor payload back to the schema-local field. The mismatch cases
report `codec.encode_mapping_mismatch` for either the outer or inner
constructor, and the range case preserves
`codec.encode_value_unrepresentable` after projection.
`../../examples/specification/run/binary-schema-sub-byte-decode/`,
`../../examples/specification/run/binary-schema-sub-byte-decode-human/`,
`../../examples/specification/run/binary-schema-sub-byte-encode/`,
`../../examples/specification/run/binary-schema-sub-byte-encode-human/`,
`../../examples/specification/run/binary-schema-sub-byte-encode-out-of-range/`,
`../../examples/specification/run/binary-schema-sub-byte-encode-out-of-range-human/`,
`../../examples/specification/run/binary-schema-sub-byte-truncated-json/`,
and
`../../examples/specification/run/binary-schema-sub-byte-truncated-human/`
pin standalone `UInt1` through `UInt7` helper behavior in JSON and human
command output. The decode cases prove one-byte-per-field low-bit masking and
structural mapping. The encode cases prove one-byte output and
`codec.encode_value_unrepresentable` for values outside the declared low-bit
range. The truncation cases prove the existing `schema.truncated_field`
diagnostic shape for a missing one-byte standalone field.

`../../examples/specification/run/binary-schema-flag8-decode/`,
`../../examples/specification/run/binary-schema-flag8-encode/`,
`../../examples/specification/run/binary-schema-flag8-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag8-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag8-encode-out-of-range/`
pin the one-byte visible flag bitset slice. The cases prove source-visible
`Flag8(bits)` decode, one-byte encode, direct structural mapping in both
directions, and the ordinary encode value-representation failure shape for
values outside the one-byte range.
`../../examples/specification/run/binary-schema-flag8-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag8-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag8-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag8-bit-index-human/` pin
checked `Flag8` helper behavior for successful raw-bit extraction, raw-bit
construction, bit reads, and bit sets plus JSON raw-bit range and human
invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag16be-decode/`,
`../../examples/specification/run/binary-schema-flag16be-encode/`,
`../../examples/specification/run/binary-schema-flag16be-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag16be-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag16be-encode-out-of-range/`
pin the two-byte big-endian visible flag bitset slice. The cases prove
source-visible `Flag16be(bits)` decode, big-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the two-byte range.
`../../examples/specification/run/binary-schema-flag16be-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag16be-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag16be-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag16be-bit-index-human/`
pin checked `Flag16be` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag16le-decode/`,
`../../examples/specification/run/binary-schema-flag16le-encode/`,
`../../examples/specification/run/binary-schema-flag16le-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag16le-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag16le-encode-out-of-range/`
pin the two-byte little-endian visible flag bitset slice. The cases prove
source-visible `Flag16le(bits)` decode, little-endian encode, direct
structural mapping in both directions, and the ordinary encode
value-representation failure shape for values outside the two-byte range.
`../../examples/specification/run/binary-schema-flag16le-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag16le-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag16le-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag16le-bit-index-human/`
pin checked `Flag16le` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag24-decode/`,
`../../examples/specification/run/binary-schema-flag24-encode/`,
`../../examples/specification/run/binary-schema-flag24-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag24-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag24-encode-out-of-range/`
pin the three-byte visible flag bitset slice. The cases prove
source-visible `Flag24be(bits)` and `Flag24le(bits)` decode, big-endian and
little-endian encode, direct structural mapping in both directions, and the
ordinary encode value-representation failure shape for values outside the
three-byte range.
`../../examples/specification/run/binary-schema-flag24-bit-helpers/` and
`../../examples/specification/run/binary-schema-flag24-helper-diagnostics-json/`
pin checked `Flag24be` and `Flag24le` helper behavior for successful raw-bit
extraction, raw-bit construction, bit reads, and bit sets plus invalid-index
and raw-bit range runtime result failures.

`../../examples/specification/run/binary-schema-flag32be-decode/`,
`../../examples/specification/run/binary-schema-flag32be-encode/`,
`../../examples/specification/run/binary-schema-flag32be-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag32be-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag32be-encode-out-of-range/`
pin the four-byte big-endian visible flag bitset slice. The cases prove
source-visible `Flag32be(bits)` decode, big-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the four-byte range.
`../../examples/specification/run/binary-schema-flag32be-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag32be-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag32be-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag32be-bit-index-human/`
pin checked `Flag32be` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag32le-decode/`,
`../../examples/specification/run/binary-schema-flag32le-encode/`,
`../../examples/specification/run/binary-schema-flag32le-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag32le-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag32le-encode-out-of-range/`
pin the four-byte little-endian visible flag bitset slice. The cases prove
source-visible `Flag32le(bits)` decode, little-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the four-byte range.
`../../examples/specification/run/binary-schema-flag32le-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag32le-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag32le-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag32le-bit-index-human/`
pin checked `Flag32le` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag40be-decode/`,
`../../examples/specification/run/binary-schema-flag40be-encode/`,
`../../examples/specification/run/binary-schema-flag40be-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag40be-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag40be-encode-out-of-range/`
pin the five-byte big-endian visible flag bitset slice. The cases prove
source-visible `Flag40be(bits)` decode, big-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the five-byte range.
`../../examples/specification/run/binary-schema-flag40be-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag40be-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag40be-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag40be-bit-index-human/`
pin checked `Flag40be` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag40le-decode/`,
`../../examples/specification/run/binary-schema-flag40le-encode/`,
`../../examples/specification/run/binary-schema-flag40le-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag40le-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag40le-encode-out-of-range/`
pin the five-byte little-endian visible flag bitset slice. The cases prove
source-visible `Flag40le(bits)` decode, little-endian encode, direct
structural mapping in both directions, and the ordinary encode
value-representation failure shape for values outside the five-byte range.
`../../examples/specification/run/binary-schema-flag40le-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag40le-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag40le-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag40le-bit-index-human/`
pin checked `Flag40le` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag48be-decode/`,
`../../examples/specification/run/binary-schema-flag48be-encode/`,
`../../examples/specification/run/binary-schema-flag48be-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag48be-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag48be-encode-out-of-range/`
pin the six-byte big-endian visible flag bitset slice. The cases prove
source-visible `Flag48be(bits)` decode, big-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the six-byte range.
`../../examples/specification/run/binary-schema-flag48be-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag48be-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag48be-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag48be-bit-index-human/`
pin checked `Flag48be` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag48le-decode/`,
`../../examples/specification/run/binary-schema-flag48le-encode/`,
`../../examples/specification/run/binary-schema-flag48le-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag48le-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag48le-encode-out-of-range/`
pin the six-byte little-endian visible flag bitset slice. The cases prove
source-visible `Flag48le(bits)` decode, little-endian encode, direct
structural mapping in both directions, and the ordinary encode
value-representation failure shape for values outside the six-byte range.
`../../examples/specification/run/binary-schema-flag48le-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag48le-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag48le-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag48le-bit-index-human/`
pin checked `Flag48le` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag56be-decode/`,
`../../examples/specification/run/binary-schema-flag56be-encode/`,
`../../examples/specification/run/binary-schema-flag56be-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag56be-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag56be-encode-out-of-range/`
pin the seven-byte big-endian visible flag bitset slice. The cases prove
source-visible `Flag56be(bits)` decode, big-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the seven-byte range.
`../../examples/specification/run/binary-schema-flag56be-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag56be-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag56be-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag56be-bit-index-human/`
pin checked `Flag56be` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag56le-decode/`,
`../../examples/specification/run/binary-schema-flag56le-encode/`,
`../../examples/specification/run/binary-schema-flag56le-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag56le-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag56le-encode-out-of-range/`
pin the seven-byte little-endian visible flag bitset slice. The cases prove
source-visible `Flag56le(bits)` decode, little-endian encode, direct
structural mapping in both directions, and the ordinary encode
value-representation failure shape for values outside the seven-byte range.
`../../examples/specification/run/binary-schema-flag56le-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag56le-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag56le-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag56le-bit-index-human/`
pin checked `Flag56le` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads, and bit sets plus JSON raw-bit range and
human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag64be-decode/`,
`../../examples/specification/run/binary-schema-flag64be-encode/`,
`../../examples/specification/run/binary-schema-flag64be-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag64be-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag64be-encode-out-of-range/`
pin the eight-byte big-endian visible flag bitset slice. The cases prove
source-visible `Flag64be(bits)` decode, big-endian encode, direct structural
mapping in both directions, and the ordinary encode value-representation
failure shape for values outside the eight-byte range.
`../../examples/specification/run/binary-schema-flag64be-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag64be-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag64be-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag64be-bit-index-human/`
pin checked `Flag64be` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads including bit index `63`, and bit sets plus
JSON raw-bit range and human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-flag64le-decode/`,
`../../examples/specification/run/binary-schema-flag64le-encode/`,
`../../examples/specification/run/binary-schema-flag64le-mapped-record-decode/`,
`../../examples/specification/run/binary-schema-flag64le-mapped-record-encode/`,
and
`../../examples/specification/run/binary-schema-flag64le-encode-out-of-range/`
pin the eight-byte little-endian visible flag bitset slice. The cases prove
source-visible `Flag64le(bits)` decode, little-endian encode, direct
structural mapping in both directions, and the ordinary encode
value-representation failure shape for values outside the eight-byte range.
`../../examples/specification/run/binary-schema-flag64le-bit-helpers/`,
`../../examples/specification/run/binary-schema-flag64le-from-bits-out-of-range-json/`,
`../../examples/specification/run/binary-schema-flag64le-bit-index-json/`, and
`../../examples/specification/run/binary-schema-flag64le-bit-index-human/`
pin checked `Flag64le` helper behavior for successful raw-bit extraction,
raw-bit construction, bit reads including bit index `63`, and bit sets plus
JSON raw-bit range and human invalid-index runtime result failures.

`../../examples/specification/run/binary-schema-reserved-bit-encode/` pins the
reserved-bit encode slice for `ReservedBits(1, 0)` followed by `UInt31be`.
The case checks complete lowercase hex output for an HTTP/2-style stream
identifier field and the `UInt31be` maximum boundary.
`../../examples/specification/run/binary-schema-reserved-byte-prefix-decode-encode/`
pins the reserved-byte-prefix slice for `ReservedBits(2, 0)` followed by
`UInt8`, including direct helpers, derived codec eligibility, lowercase hex
output, and the visible-field range failure.
`../../examples/specification/run/binary-schema-reserved-nine-bit-prefix-decode-encode/`
pins the same two-byte byte-prefix helper route for `ReservedBits(9, 0)`
followed by `UInt8`; the adjacent JSON cases pin the reserved-bit mismatch
and truncation projections for that layout. The adjacent checker case
`../../examples/specification/check/schema-reserved-bit-encode-diagnostics/`
asserts `schema.reserved_bits_encode` for a non-byte-aligned reserved-bit
shape outside the supported encode layouts.
`../../examples/specification/check/schema-reserved-bit-layout-diagnostics/`
and `../../examples/specification/check/schema-reserved-bit-layout-human/`
pin unsupported suffix, prefix, and standalone `ReservedBits(width, value)`
layout diagnostics with schema, field, reserved width, adjacent visible width,
and supported layout family context.
`../../examples/specification/run/binary-schema-byte-aligned-reserved-encode/`
pins byte-aligned reserved-bit encode: the helper omits the reserved field
from the source value record and writes the declared fixed bytes in
declaration order.
`../../examples/specification/run/binary-schema-middle-reserved-decode-encode/`
pins a non-byte-aligned middle reserved-bit storage unit: decode omits the
middle `ReservedBits(width, value)` field while preserving the adjacent
visible fields, encode writes the declared reserved bits between those
visible fields, and visible out-of-range encode values keep the ordinary
`codec.encode_value_unrepresentable` shape. The adjacent
`../../examples/specification/run/binary-schema-middle-reserved-json/` case
pins `schema.reserved_bits_mismatch` field path, byte offset, bit width,
expected value, actual value, and byte preview details for the same middle
layout.
`../../examples/specification/run/binary-schema-byte-interleaved-middle-reserved-decode-encode/`
also pins decode, encode, derived codec eligibility, and visible field encode
diagnostics for the byte-interleaved middle reserved-bit layout. The adjacent
`../../examples/specification/run/binary-schema-byte-interleaved-middle-reserved-json/`
case pins the reserved-bit mismatch diagnostic for that layout.
`../../examples/specification/run/binary-schema-prefix-reserved-group-decode-encode/`
pins a non-byte-aligned reserved prefix followed by two visible `UIntN`
fields in the same one-byte storage unit: decode omits the
`ReservedBits(width, value)` field while preserving both visible fields,
encode writes the declared reserved bits before those visible fields, and
visible out-of-range encode values keep the ordinary
`codec.encode_value_unrepresentable` shape.
`../../examples/specification/run/binary-schema-prefix-reserved-two-byte-group-decode-encode/`
and
`../../examples/specification/run/binary-schema-prefix-reserved-two-byte-group-json/`
pin the corresponding two-byte big-endian reserved prefix group: decode and
encode preserve the two visible `UIntN` fields in declaration order, omit the
reserved prefix from values, report visible-field range failures on either
visible field path, and keep `schema.reserved_bits_mismatch` on the reserved
field path when the high reserved bits differ.
`../../examples/specification/run/binary-schema-prefix-reserved-low-width-two-byte-group-decode-encode/`
pins the minimum reserved-width boundary for that two-byte form: a one-bit
reserved prefix followed by `UInt7` and `UInt8` decodes and encodes through
the shared big-endian storage unit, omits the reserved prefix from mapped
values, and keeps visible-field range failures on either visible field path.
`../../examples/specification/run/binary-schema-prefix-reserved-three-byte-group-decode-encode/`
and
`../../examples/specification/run/binary-schema-prefix-reserved-three-byte-group-json/`
pin the three-byte big-endian reserved prefix group with a seventeen-bit
reserved prefix followed by two visible sub-byte `UIntN` fields: decode and
encode preserve the visible fields in declaration order, omit the reserved
prefix from values, report visible-field range failures on either visible
field path, and keep `schema.reserved_bits_mismatch` on the reserved field
path when the high reserved bits differ.
`../../examples/specification/run/binary-schema-prefix-reserved-four-byte-group-decode-encode/`,
`../../examples/specification/run/binary-schema-prefix-reserved-four-byte-group-json/`,
`../../examples/specification/run/binary-schema-prefix-reserved-four-byte-group-truncated-json/`,
`../../examples/specification/run/binary-schema-prefix-reserved-four-byte-group-high-encode-out-of-range/`,
and
`../../examples/specification/run/binary-schema-prefix-reserved-four-byte-group-low-encode-out-of-range/`
pin the four-byte big-endian reserved prefix group with a twenty-five-bit
reserved prefix followed by two visible sub-byte `UIntN` fields: decode and
encode preserve the visible fields in declaration order, omit the reserved
prefix from values, report visible-field range failures on each visible field
path, and keep `schema.reserved_bits_mismatch` on the reserved field path
when the high reserved bits differ. The truncated case keeps
`schema.truncated_field` on the reserved field path when the shared storage
unit is incomplete.
`../../examples/specification/run/binary-schema-prefix-reserved-five-byte-group-decode-encode/`,
`../../examples/specification/run/binary-schema-prefix-reserved-five-byte-group-json/`,
`../../examples/specification/run/binary-schema-prefix-reserved-five-byte-group-human/`,
and
`../../examples/specification/run/binary-schema-prefix-reserved-five-byte-group-encode-out-of-range/`
pin the five-byte big-endian reserved prefix group with a thirty-three-bit
reserved prefix followed by two visible sub-byte `UIntN` fields. The cases
cover decode and encode, JSON and human reserved-bit mismatch diagnostics,
and visible-field encode range failures.
`../../examples/specification/run/binary-schema-prefix-reserved-six-byte-group-decode-encode/`,
`../../examples/specification/run/binary-schema-prefix-reserved-six-byte-group-json/`,
`../../examples/specification/run/binary-schema-prefix-reserved-six-byte-group-human/`,
and
`../../examples/specification/run/binary-schema-prefix-reserved-six-byte-group-encode-out-of-range/`
pin the six-byte big-endian reserved prefix group with a forty-one-bit
reserved prefix followed by two visible sub-byte `UIntN` fields. The cases
mirror the five-byte coverage and keep the reserved prefix representation-only
while preserving visible-field order.
`../../examples/specification/run/binary-schema-prefix-reserved-byte-group-decode-encode/`,
`../../examples/specification/run/binary-schema-prefix-reserved-byte-group-json/`,
and
`../../examples/specification/run/binary-schema-prefix-reserved-byte-group-encode-out-of-range/`
pin the byte-aligned reserved prefix variant of the same two-byte shape:
`ReservedBits(8, value)` occupies the high byte, two visible sub-byte `UIntN`
fields complete the low byte, the reserved field stays representation-only,
and mismatch and visible range failures keep the same structured diagnostics.
`../../examples/specification/run/binary-schema-split-reserved-decode-encode/`
pins a shared storage byte containing multiple non-byte-aligned
`ReservedBits(width, value)` fields: decode omits both reserved fields while
preserving the visible fields, encode writes both declared reserved values in
declaration order, and visible out-of-range encode values keep the ordinary
`codec.encode_value_unrepresentable` shape.
`../../examples/specification/run/binary-schema-interleaved-reserved-decode-encode/`
and
`../../examples/specification/run/binary-schema-interleaved-reserved-json/`
pin the one-reserved-field companion shape: one
`ReservedBits(width, value)` field is interleaved with three visible `UIntN`
fields in one shared storage byte, decode and encode preserve only the
visible fields, and mismatch diagnostics stay at the reserved field path.

`../../examples/specification/run/binary-schema-byteview-encode/` and
`../../examples/specification/run/binary-schema-byteview-encode-length-mismatch/`
pin length-bounded `ByteView(length_field)` encode. The passing case writes
the explicit length field and the bounded bytes from the supplied view into
one immutable `ByteChunk`. The failing case matches the returned
`EncodeError` and asserts `codec.encode_value_unrepresentable`, the schema
field path, and the view-count mismatch reason. The derived codec boundary
case
`../../examples/specification/run/derived-codec-byteview-encode-boundary/`
pins the same helper eligibility through `derive encode`.
The repeated byte-view encode cases listed in
`../../examples/specification/README.md` pin generated encode for
`Repeat(count_field, ByteView(length_field))`. The passing case writes the
count field, length field, and each element's bounded bytes in order; the
failing cases assert the encode error id, repeated field path plus element
index, view-count mismatch reason, and `derive encode` error projection.
`../../examples/specification/run/binary-schema-byteview-subtract-decode/`,
`../../examples/specification/run/binary-schema-byteview-subtract-negative-json/`,
`../../examples/specification/run/binary-schema-byteview-subtract-truncated-json/`,
`../../examples/specification/run/binary-schema-byteview-subtract-encode/`, and
`../../examples/specification/run/binary-schema-byteview-subtract-encode-length-mismatch/`
pin the same boundary for `ByteView(length - padding_length)`, including
negative computed lengths, payload truncation, direct helper encode mismatch,
and derived codec encode success.
`../../examples/specification/run/binary-schema-byteview-product-decode/`,
`../../examples/specification/run/binary-schema-byteview-product-truncated-json/`,
`../../examples/specification/run/binary-schema-byteview-product-encode/`, and
`../../examples/specification/run/binary-schema-byteview-product-encode-length-mismatch/`
pin `ByteView(row_count * column_count)` decode, short-input failure, derived
codec encode success, and direct helper encode mismatch.
`../../examples/specification/run/binary-schema-byteview-quotient-decode/`,
`../../examples/specification/run/binary-schema-byteview-quotient-encode/`, and
`../../examples/specification/run/binary-schema-byteview-quotient-encode-length-mismatch/`
pin `ByteView(total_length / chunk_count)` decode, derived codec encode
success, and direct helper encode mismatch.
`../../examples/specification/run/binary-schema-repeat-quotient-decode/`,
`../../examples/specification/run/binary-schema-repeat-quotient-encode/`,
`../../examples/specification/run/binary-schema-repeat-quotient-encode-count-mismatch/`,
and
`../../examples/specification/run/binary-schema-repeat-quotient-division-by-zero-json/`
pin `Repeat(total_count / group_count, UInt16be)` decode, encode, direct
helper encode mismatch, and division-by-zero diagnostics.

`../../examples/specification/run/binary-schema-repeat-truncated-json/` and
`../../examples/specification/run/binary-schema-repeat-truncated-human/` pin
repeated primitive decode truncation. The JSON case asserts
`schema.truncated_field` with the repeated field path plus the failed element
`index` segment. The human case keeps the primary message focused on the
missing byte offset and carries readiness, byte counts, nearby bytes, and the
same indexed field path in related notes.

`../../examples/specification/run/binary-schema-packed-reserved-encode/`,
`../../examples/specification/run/binary-schema-packed-reserved-four-byte-encode/`,
`../../examples/specification/run/binary-schema-packed-reserved-three-byte-encode/`,
`../../examples/specification/run/binary-schema-packed-reserved-four-byte-encode-out-of-range/`,
and
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-encode-out-of-range/`
pin packed reserved-bit encode: the helper writes high reserved bits from the
declaration and low visible bits from the source value record in the shared
one-byte, two-byte, three-byte, or four-byte big-endian storage unit, and
reports
`codec.encode_value_unrepresentable` against the visible low-bit field when
the value does not fit.
`../../examples/specification/run/binary-schema-packed-reserved-suffix-encode/`,
`../../examples/specification/run/binary-schema-packed-reserved-suffix-encode-out-of-range/`,
`../../examples/specification/run/binary-schema-packed-reserved-four-byte-encode/`,
`../../examples/specification/run/binary-schema-packed-reserved-three-byte-encode/`,
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-encode/`,
and
`../../examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-encode-out-of-range/`
pin the packed reserved suffix encode slice. The helper writes the visible
value in the high bits, writes the declared reserved value in the low bits for
one-byte, two-byte, three-byte, and four-byte shared storage units, and reports
`codec.encode_value_unrepresentable` against the visible field when the input
record value exceeds the field range.
`../../examples/specification/run/binary-schema-middle-reserved-decode-encode/`
also pins middle reserved-bit encode: the helper writes the declared reserved
value between adjacent visible fields in the shared storage unit and reports
`codec.encode_value_unrepresentable` against the adjacent visible field when
the input record value exceeds that field's range.
`../../examples/specification/run/binary-schema-prefix-reserved-group-decode-encode/`
pins the matching reserved prefix group encode slice for one-byte storage:
the helper writes the declared reserved value before two visible `UIntN`
fields and reports `codec.encode_value_unrepresentable` against the visible
field whose source value is outside its bit range. The two-byte companion
case
`../../examples/specification/run/binary-schema-prefix-reserved-two-byte-group-decode-encode/`
uses the same encode rule for a shared two-byte big-endian storage unit and
checks both visible field paths for range failures.
`../../examples/specification/run/binary-schema-prefix-reserved-byte-group-encode-out-of-range/`
pins the byte-aligned reserved prefix companion case, keeping the
`codec.encode_value_unrepresentable` field path on the out-of-range visible
field.
`../../examples/specification/run/binary-schema-split-reserved-decode-encode/`
also pins split reserved-bit encode: the helper writes multiple declared
reserved values in one shared storage byte with adjacent visible `UIntN`
fields and reports `codec.encode_value_unrepresentable` against the visible
field whose input value exceeds its range.
`../../examples/specification/run/binary-schema-interleaved-reserved-decode-encode/`
pins the single reserved-field variant with three visible `UIntN` fields in
the same shared storage byte.

`../../examples/specification/run/binary-schema-closed-dispatch-encode/`
pins the closed dispatch encode helper slice. The passing cases select
`UInt8`, `UInt16be`, `UInt24be`, and `UInt32be` payload widths from an earlier
tag field and write one `ByteChunk` in declaration order.
`../../examples/specification/run/binary-schema-closed-dispatch-nested-encode/`
pins same-module nested payload encode for a closed dispatch case.
`../../examples/specification/run/binary-schema-recursive-closed-dispatch-encode/`
pins same-module recursive payload encode for a length-bounded closed dispatch
case whose selected mappings cover every dispatch tag.
`../../examples/specification/run/binary-schema-recursive-extension-dispatch-encode/`
pins same-module recursive known payload encode for a length-bounded extension
dispatch case while preserving the explicit length check.
`../../examples/specification/run/binary-schema-imported-recursive-dispatch-encode/`
pins the same recursive closed and extension payload encode boundaries when
the recursive payload schema is public and named through a written `use` path,
including extension unknown raw payload preservation.
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`
pins closed and extension-tolerant nested payload encode through the generated
schema helper path, including byte-aligned reserved fields and little-endian
primitive output.
`../../examples/specification/run/binary-schema-imported-closed-dispatch-nested-encode/`
pins public imported nested payload encode for a closed dispatch case.
`../../examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-encode/`
pins direct generated encode over selected mappings that project one target
record shape back to mixed primitive and nested closed dispatch payload cases,
including primitive and nested helper range failures, unknown tags, and
tag/payload mismatches.
`../../examples/specification/run/binary-schema-closed-dispatch-encode-unknown-tag/`
asserts `codec.dispatch_unknown_tag` when the tag value has no closed case.
`../../examples/specification/run/binary-schema-closed-dispatch-encode-out-of-range/`
asserts `codec.encode_value_unrepresentable` against the selected `UInt8`
payload case.
`../../examples/specification/run/binary-schema-recursive-dispatch-length-encode-diagnostic-json/`
asserts `codec.dispatch_length_mismatch` when recursive closed-dispatch encode
produces a payload byte count that differs from the supplied length field.
`../../examples/specification/run/binary-schema-recursive-extension-dispatch-length-encode-diagnostic-json/`
asserts the same length mismatch diagnostic for a recursive extension-dispatch
known payload.
`../../examples/specification/run/binary-schema-imported-recursive-dispatch-length-encode-diagnostic-json/`
and
`../../examples/specification/run/binary-schema-imported-recursive-extension-dispatch-length-encode-diagnostic-json/`
assert the same explicit length check for public imported recursive closed and
extension dispatch payload schemas.

`../../examples/specification/run/binary-schema-extension-dispatch-encode/`
pins the extension-tolerant dispatch encode helper slice. The passing cases
write a known primitive payload selected by the visible tag field and preserve
unknown raw bounded payload bytes when the unknown payload tag matches the
visible tag value.
`../../examples/specification/run/binary-schema-extension-dispatch-nested-encode/`
pins same-module nested payload encode through
`SchemaDispatchPayload::Known`.
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`
also pins that extension-tolerant known nested payload encode uses the
generated schema helper path.
`../../examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode/`
pins public imported nested payload encode through
`SchemaDispatchPayload::Known`.
`../../examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode-unknown/`
pins unknown raw payload preservation when the known cases name public
imported nested payload schemas.
`../../examples/specification/run/binary-schema-extension-dispatch-encode-mismatch/`
asserts `codec.dispatch_mismatch` when the visible tag field selects a known
case but the payload field supplies `Unknown`.
`../../examples/specification/run/binary-schema-extension-dispatch-encode-tag-mismatch/`
asserts `codec.dispatch_mismatch` when an unknown payload variant carries a
tag that differs from the visible tag field.
`../../examples/specification/run/binary-schema-extension-dispatch-encode-out-of-range/`
asserts `codec.encode_value_unrepresentable` against the selected `UInt16be`
payload case.
`../../examples/specification/run/binary-schema-extension-dispatch-encode-length-mismatch/`
asserts `codec.dispatch_length_mismatch` when the earlier length field does
not match the emitted payload byte count.
`../../examples/specification/run/binary-schema-dispatch-nested-encode-failure/`
asserts that nested payload encode failures report
`codec.encode_value_unrepresentable` while keeping the nested schema field
path.
`../../examples/specification/run/binary-schema-imported-dispatch-nested-encode-failure/`
asserts the same nested field path behavior through a public imported payload
schema.
`../../examples/specification/run/derived-codec-imported-nested-dispatch-encode-boundary/`
pins the same public imported nested payload helper eligibility when reached
through a `derive encode` codec boundary, including helper error projection to
`EncodeStep::Invalid`.
`../../examples/specification/run/derived-codec-mixed-dispatch-selected-mapping-encode-boundary/`
pins the same mixed dispatch selected mapping encode boundary through
`derive encode`, including nested helper and dispatch error projection to
`EncodeStep::Invalid`.

`../../examples/specification/run/binary-schema-closed-dispatch-decode/`,
`../../examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
`../../examples/specification/run/binary-schema-imported-closed-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-imported-recursive-dispatch-decode/`,
`../../examples/specification/run/binary-schema-dispatch-nested-failure-json/`,
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-failure-json/`,
`../../examples/specification/run/binary-schema-imported-dispatch-nested-failure-json/`,
`../../examples/specification/run/binary-schema-imported-recursive-dispatch-failure-json/`,
`../../examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
and
`../../examples/specification/run/binary-schema-closed-dispatch-unknown-human/`
pin the narrow closed dispatch slice. The passing case decodes a known tag and
selected primitive payload as ordinary `Int` fields; the nested passing cases
decode selected same-module and public imported payload schemas as
record-shaped fields. The general helper passing case proves selected nested
payload schemas keep fixed-field validation, byte-aligned reserved fields, and
little-endian primitive reads when reached through closed or extension
dispatch. The recursive closed-dispatch case pins a same-module recursive
payload decoded through a length-bounded closed dispatch, selected mappings,
the generated helper path, and a non-recursive base case. The recursive
extension-dispatch case pins the same known-payload helper path while unknown
tags preserve bounded raw payload bytes. The imported recursive decode case
pins the same closed and extension dispatch behavior when the recursive
payload schema is public and named through a written `use` path. The nested failure
cases pin the outer dispatch field path, nested schema field path, and
absolute byte offset, including fixed-field mismatch diagnostics produced by
the nested helper. The recursive failure cases pin that same path prefix for a
nested length-boundary failure, including the imported recursive path. The
unknown-tag failing cases assert
`schema.dispatch_unknown_tag`, the dispatch byte offset, structured field path,
decoded tag field and value, expected tag values, structured byte preview
fields, and focused human related notes.
`../../examples/specification/run/derived-codec-imported-nested-dispatch-decode-boundary/`
pins the same public imported nested payload helper eligibility when reached
through a `derive decode` codec boundary, including the returned
`DecodeStep` consumed count.
`../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`
pins the static boundary for nested dispatch payload schema names, including
missing names, non-schema names, private imported schemas, self references
outside the eligible recursive length-bounded dispatch slice, schemas outside
the generated helper slice, forward references, and incompatible payload
shapes.
`../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
and
`../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`
pin a resolved binary payload schema whose forward `ByteView` length reference
keeps the nested payload outside generated decode and encode helper
eligibility, including structured helper-boundary fields, human related notes,
and derived codec helper rejection for the parent dispatch schemas.
`../../examples/specification/check/binary-schema-recursive-dispatch-payload-diagnostics/`
pins the remaining self-reference rejection when a recursive closed dispatch
is not length-bounded, or when an imported recursive payload is referenced
outside the selected length-bounded mapping boundary.
`../../examples/specification/check/binary-schema-mixed-dispatch-selected-mapping-diagnostics/`
pins the remaining `schema.dispatch_payload` rejection when mixed dispatch
payload shapes use selected mappings keyed by a field other than the dispatch
tag.

`../../examples/specification/run/binary-schema-extension-dispatch-decode/`,
`../../examples/specification/run/binary-schema-extension-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
`../../examples/specification/run/binary-schema-imported-extension-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-recursive-extension-dispatch-decode/`,
`../../examples/specification/run/binary-schema-extension-dispatch-unknown/`,
`../../examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
`../../examples/specification/run/binary-schema-imported-extension-dispatch-nested-unknown/`,
and
`../../examples/specification/run/binary-schema-extension-dispatch-length-human/`
pin the narrow extension-tolerant dispatch slice. The known case decodes the
selected exact-width, same-module nested, or public imported nested schema
payload into `SchemaDispatchPayload::Known`. The recursive case decodes known
recursive payloads through the same helper path. The unknown cases preserve
the decoded tag and a bounded raw `ByteView` without reporting
`schema.dispatch_unknown_tag`. The malformed structural case still reports
`schema.length_out_of_bounds` when the decoded length cannot be sliced from
closed input.
`../../examples/specification/run/binary-schema-general-helper-roundtrip/`
combines supported generated helper fields in one non-HTTP schema and checks
that successful decode followed by encode preserves the same bytes, including
calls routed through the derived codec item name. The same case also checks
short-input decode readiness plus decode and encode helper failures observed
through that derived codec item name.

## Stream Adapter Event Boundary

The executable specification case
`../../examples/specification/run/stream-adapter-event-boundary/` covers the
implemented source-level adapter boundary for decoded stream work. The example
declares ordinary `StreamEvent` and `ResponseAction` ADTs, calls a plain
handler directly with a synthesized event and explicit state record, routes
another event through an existing channel under the `concurrency` effect, and
checks that the handler returns response-action intent values plus the next
state. The actions describe send-bytes, end-stream, reset-stream, and decline
intent as values for an adapter to interpret; the handler does not call socket
or `net::send_chunk` APIs.

The executable specification case
`../../examples/specification/run/transport-socket-boundary/` covers the first
fixture-backed socket API slice. It creates a source-visible `NetListener`,
accepts a distinct `NetStream`, reads one immutable `ByteChunk`, writes one
immutable `ByteChunk`, and records host transport events while keeping all
calls under the coarse `net` effect. The
`../../examples/specification/run/transport-socket-write-chunks-boundary/`
case writes a source-owned `List<ByteChunk>` to the same stream in list order
with `net::write_chunks`. The matching
`../../examples/specification/check/transport-socket-write-chunks-effects/`
case pins that `net::write_chunks` requires the same `net` effect. The
`../../examples/specification/run/transport-socket-optional-accept-boundary/`
case covers `net::accept_or_end` returning `Some(stream)` and using that
stream with the existing read behavior, while
`../../examples/specification/run/transport-socket-optional-accept-clean-end/`
covers clean listener end returning `None`.
`../../examples/specification/run/transport-socket-listener-close-boundary/`
covers `net::close_listener` recording an ordered fixture listener-close event,
leaving an already accepted stream readable, and making a later optional
accept fail as a runtime transport failure.
`../../examples/specification/run/transport-socket-accept-until-boundary/`
covers `net::accept_until` returning `Some(stream)` when a fixture accepts
before the deadline, while
`../../examples/specification/run/transport-socket-accept-until-deadline/`
covers fixture-reported accept deadline expiry returning `None`.
The matching
`../../examples/specification/run/transport-socket-accept-until-cancellable-boundary/`
case covers `net::accept_until_cancellable` returning
`AcceptStream(stream)`,
`../../examples/specification/run/transport-socket-accept-until-cancellable-clean-end/`
covers clean listener end returning `AcceptEnd`,
`../../examples/specification/run/transport-socket-accept-until-cancellable-deadline/`
covers fixture-reported accept deadline expiry returning
`AcceptDeadlineExpired`,
`../../examples/specification/run/transport-socket-accept-until-cancellable-expired/`
covers an already expired supplied deadline returning
`AcceptDeadlineExpired`, and
`../../examples/specification/run/transport-socket-accept-until-cancellable-cancelled/`
covers token cancellation returning `AcceptCancelled`.
`../../examples/specification/run/transport-socket-read-until-boundary/`
covers `net::read_chunk_until` returning `Some(bytes)` when a fixture stream
yields a chunk before the deadline,
`../../examples/specification/run/transport-socket-read-until-expired/`
covers an already expired supplied deadline returning `None`,
`../../examples/specification/run/transport-socket-read-until-deadline/`
covers fixture-reported read deadline expiry returning `None`, and
`../../examples/specification/run/transport-socket-read-until-clean-end/`
covers clean stream end returning `None`. The matching
`../../examples/specification/run/transport-socket-read-until-cancellable-boundary/`
case covers `net::read_chunk_until_cancellable` returning `ReadChunk(bytes)`,
`../../examples/specification/run/transport-socket-read-until-cancellable-clean-end/`
covers clean stream end returning `ReadEnd`,
`../../examples/specification/run/transport-socket-read-until-cancellable-deadline/`
covers fixture-reported read deadline expiry returning `ReadDeadlineExpired`,
and
`../../examples/specification/run/transport-socket-read-until-cancellable-cancelled/`
covers token cancellation returning `ReadCancelled`. The matching
`../../examples/specification/run/transport-socket-write-until-cancellable-boundary/`
case covers `net::write_chunk_until_cancellable` returning `WriteCompleted`,
`../../examples/specification/run/transport-socket-write-until-cancellable-deadline/`
covers fixture-reported write deadline expiry returning
`WriteDeadlineExpired`, and
`../../examples/specification/run/transport-socket-write-until-cancellable-cancelled/`
covers token cancellation returning `WriteCancelled`.
`../../examples/specification/run/transport-socket-write-until-cancellable-production-outcomes/`
covers the same success, deadline, and cancellation outcomes in the
production-loopback runtime. The matching
`../../examples/specification/check/transport-socket-effects/` case pins
missing-effect diagnostics for the socket calls, including the optional
clean-end listener accept and stream read,
`../../examples/specification/check/transport-socket-optional-accept-effects/`
pins the optional accept directly, and
`../../examples/specification/check/transport-socket-accept-until-effects/`
pins that deadline-aware accept requires both `net` and `time`, and
`../../examples/specification/check/transport-socket-accept-until-cancellable-effects/`
pins that cancellable deadline-aware accept requires both `net` and `time`,
and
`../../examples/specification/check/transport-socket-read-until-effects/`
pins that deadline-aware read requires both `net` and `time`, and
`../../examples/specification/check/transport-socket-read-until-cancellable-effects/`
pins the same effect boundary for cancellable deadline-aware reads, and
`../../examples/specification/check/transport-socket-write-until-cancellable-effects/`
pins the same effect boundary for cancellable deadline-aware writes, and
`../../examples/specification/check/transport-socket-clean-end-effects/` pins
the optional clean-end read directly. The
`../../examples/specification/check/transport-socket-listener-close-effects/`
case pins that explicit listener close requires the `net` effect. The
`../../examples/specification/check/socket-stream-close-effects/` case pins
that explicit stream close requires the `net` effect. The
`../../examples/specification/run/transport-socket-listener-close-accept-until-failure-json/`,
`../../examples/specification/run/transport-socket-listener-close-accept-until-cancellable-failure-json/`,
and
`../../examples/specification/run/transport-socket-listener-close-failure-json/`
cases keep accept-after-listener-close and forced listener-close failures on
the runtime transport-failure surface. The
`../../examples/specification/run/transport-socket-listener-close-record-failure-json/`
case keeps listener-close event-recording failure on the same surface.
`../../examples/specification/run/transport-socket-read-failure-human/`,
`../../examples/specification/run/transport-socket-read-failure-json/`,
`../../examples/specification/run/transport-socket-read-or-end-failure-json/`,
`../../examples/specification/run/transport-socket-optional-accept-failure-json/`,
`../../examples/specification/run/transport-socket-accept-until-failure-json/`,
`../../examples/specification/run/transport-socket-accept-until-cancellable-failure-json/`,
`../../examples/specification/run/transport-socket-read-until-failure-json/`,
`../../examples/specification/run/transport-socket-write-until-cancellable-failure-json/`,
`../../examples/specification/run/transport-socket-write-failure-human/`, and
`../../examples/specification/run/transport-socket-write-failure-json/` cases
show accept, read, and write failures as runtime transport failures, not
schema, codec, or peer protocol diagnostics.

The executable specification cases
`../../examples/specification/run/transport-boundary/`,
`../../examples/specification/run/transport-deadline/`,
`../../examples/specification/run/transport-cancellable-wait/`, and
`../../examples/specification/check/transport-cancellable-wait-effects/`
cover descriptor-backed time waits, relative deadlines, and source-visible
`CancelToken` values under the existing `time` effect. The
`../../examples/specification/run/transport-cancel-token-status/` and
`../../examples/specification/check/transport-cancel-token-status-effects/`
cases pin cancellation-token status observation before and after
`time::cancel` and require the same `time` effect. The
`../../examples/specification/run/transport-cancellable-wait-outcome/`,
`../../examples/specification/run/transport-cancellable-wait-outcome-deadline/`,
and
`../../examples/specification/check/transport-cancellable-wait-outcome-effects/`
cases cover the value-returning wait that lets adapter code translate
completion, deadline expiry, and cancellation into ordinary source decisions.
The
`../../examples/specification/run/stream-adapter-cancellable-routing/`,
`../../examples/specification/run/stream-adapter-cancellable-routing-deadline/`,
`../../examples/specification/run/stream-adapter-cancellable-channel-first-routing/`,
and
`../../examples/specification/check/stream-adapter-cancellable-routing-effects/`
and
`../../examples/specification/check/stream-adapter-cancellable-channel-first-routing-effects/`
cases compose those wait outcomes with channel-routed `StreamInput` values and
ordinary response action values. The cancellable channel-first case routes
ordinary stream inputs through receiver-list `channel::select_many_timeout`
before translating completed wait, deadline-expired, and cancelled outcomes.
The receiver-list timeout cancellation cases use
`../../examples/specification/run/channel-select-many-timeout-cancellable/`,
`../../examples/specification/run/channel-select-many-timeout-cancellable-forced-cancel/`,
and
`../../examples/specification/check/channel-select-many-timeout-cancellable-effects/`
to pin the source-visible `channel::select_many_timeout_cancellable` boundary:
ready receiver priority returns `Ok(Some(...))`, timeout returns `Ok(None)`,
and token cancellation returns `Err(SelectError)` under the existing `time`
and `concurrency` effects. The two-receiver timeout cancellation case uses
`../../examples/specification/run/channel-select-timeout-cancellable/` and
`../../examples/specification/check/channel-select-timeout-cancellable-effects/`
to pin the matching `channel::select_timeout_cancellable` boundary.
The main routing case also pins those three wait paths in one fixture output;
the deadline case pins the global host-forced deadline expiry fixture.
Together they show that these outcomes become adapter decisions rather than
runtime failures, and that the adapter declares both `time` and `concurrency`
while the handler boundary stays free of transport effects.
The
`../../examples/specification/run/transport-timeout-expired-json/`,
`../../examples/specification/run/transport-deadline-expired-json/`,
`../../examples/specification/run/transport-cancellable-wait-deadline-expired-json/`,
and
`../../examples/specification/run/transport-cancellable-wait-cancelled-json/`
cases show timeout expiry, deadline expiry, and cancellable-wait cancellation
as runtime transport failures, not schema, codec, or peer protocol
diagnostics.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-routing-context/` covers
the socket stream adapter task boundary using one anonymous context record.
The adapter gathers the stream event, state, route, and trace fields into that
record, calls `task::spawn_with<Result, Context>(handler, context)`, joins the
task, and then projects the ordinary response value back into adapter-owned
socket output. The handler remains socket-free and receives exactly one
context parameter.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-clean-end/` covers the
clean stream-end adapter slice. Adapter-owned source reads one or more chunks
with `net::read_chunk_or_end`, observes clean end as `None`, translates that
condition into the ordinary `StreamInput.End` value, routes stream inputs
through an existing channel, calls a pure handler, and projects response
actions back to ordered `net::write_chunk` calls. Forced read failure on the
same optional read path remains a runtime transport failure.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-owned-lifecycle/`
covers the listener-to-clean-stream-end ownership boundary in one adapter
path. The adapter creates and owns the `NetListener`, accepts an optional
`NetStream` with `net::accept_or_end`, reads accepted stream chunks until
clean end with `net::read_chunk_or_end`, routes ordinary stream input values
through a channel, calls a pure handler without exposing socket handles, and
projects `SendBytes` response actions back to ordered `net::write_chunk`
calls. The adapter declares the existing coarse `net` and `concurrency`
effects; the handler remains free of `net` calls. The matching
`../../examples/specification/check/socket-stream-adapter-owned-lifecycle-effects/`
case pins that missing either adapter effect is rejected while the handler
boundary remains transport-free.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-close-lifecycle/`
covers explicit adapter-owned stream close after clean stream end. The adapter
reads chunks until `net::read_chunk_or_end` returns `None`, routes ordinary
stream input values through a channel, applies handler-produced `SendBytes`
actions as ordered `net::write_chunk` calls, and then calls
`net::close_stream`. The fixture event log pins the close event after the final
write while the pure handler remains free of socket handles and `net` calls.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-production-lifecycle/`
covers the opt-in production-loopback lifecycle on the same public socket
calls. Adapter-owned source listens, accepts, reads, routes ordinary
`StreamInput` values through a channel, calls a pure handler, writes ordered
response bytes, and closes the stream while the runtime captures the
client-observed bytes. The executable specification case
`../../examples/specification/run/socket-stream-adapter-production-two-streams/`
uses the same adapter handler/action boundary for two independent production
loopback streams accepted from one listener. Each stream is routed through the
ordinary `StreamInput` handler path with independent state, only ordered
`SendBytes` actions become socket writes, both streams close, both
client-observed byte sequences are captured, and a final optional accept
observes clean listener end. The executable specification case
`../../examples/specification/run/socket-stream-adapter-production-drain-lifecycle/`
uses optional accept as a listener-drain loop rather than a fixed stream-count
entry path. Each accepted production stream is owned by adapter code, routed
through the same ordinary handler/action boundary, written and closed in
order, and the loop stops only when `net::accept_or_end` reports clean
listener end. The matching
`../../examples/specification/run/socket-stream-adapter-production-drain-read-failure-json/`
case forces a production read failure after accept and checks that the command
surface remains a runtime transport failure without response writes or stream
close. The executable specification case
`../../examples/specification/run/socket-stream-adapter-production-deadline-lifecycle/`
uses the same production-loopback handler/action boundary through
deadline-aware `net::accept_until` and `net::read_chunk_until` calls. The
adapter accepts, reads until clean stream end becomes `None`, writes the
ordered response, closes the stream, and then observes clean listener end
through a following deadline-aware accept. The executable specification cases
`../../examples/specification/run/socket-stream-adapter-production-cancellable-deadline-lifecycle/`
and
`../../examples/specification/run/socket-stream-adapter-production-cancellable-deadline-outcomes/`
cover the same production-loopback handler/action boundary through
cancellable deadline-aware accept and read outcomes. The adapter accepts with
`net::accept_until_cancellable`, reads with
`net::read_chunk_until_cancellable`, routes `ReadChunk` and `ReadEnd` as
ordinary `StreamInput` values through a channel, translates accept/read
deadline and cancellation outcomes into adapter decisions, writes only
`SendBytes` responses, closes owned streams, observes clean listener end, and
closes the listener. The matching
`../../examples/specification/check/socket-stream-adapter-production-cancellable-deadline-lifecycle-effects/`
case pins the `net`, `time`, and `concurrency` adapter effect boundary while
the handler remains free of transport, time, and channel effects. The existing
`../../examples/specification/run/socket-stream-adapter-production-accept-until-failure-json/`
and
`../../examples/specification/run/socket-stream-adapter-production-read-until-failure-json/`
cases keep forced deadline-aware production accept and read failures on the
runtime transport-failure surface. The executable specification case
`../../examples/specification/run/socket-stream-adapter-production-close-failure-json/`
uses the same production-loopback handler/action boundary and forces a close
failure after the adapter has accepted a stream, routed the ordinary
`StreamInput` value, and projected the handler's ordered `SendBytes` response
to the stream. The JSON result stays a runtime transport failure, and the
event log pins that no close event is recorded after the forced failure. The
executable specification case
`../../examples/specification/run/transport-socket-production-two-streams/`
uses one production loopback listener to accept two independent streams, read,
write, and close each stream, and then observe clean listener end through
`net::accept_or_end`. The
`../../examples/specification/run/transport-socket-production-listener-close/`
case closes the production listener after accepting a stream, proves that the
accepted stream remains readable and writable, closes that stream, and then
checks that a later accept fails as a runtime transport failure. The matching
`../../examples/specification/run/transport-socket-production-listen-failure-json/`
case pins invalid production listen addresses as runtime transport failures.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-deadline-lifecycle/`
covers the deadline-aware accepted-stream ownership boundary. The adapter
accepts a stream with `net::accept_until`, reads chunks with
`net::read_chunk_until` until a read attempt returns `None` for deadline
expiry, routes ordinary `StreamInput` values through a channel, calls a pure
handler without exposing socket handles, and projects only `SendBytes`
response actions back to ordered `net::write_chunk` calls. The matching
`../../examples/specification/check/socket-stream-adapter-deadline-lifecycle-effects/`
case pins the composed `net`, `time`, and `concurrency` effect boundary while
the handler stays free of transport effects.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-cancellable-lifecycle/`
covers the cancellable accepted-stream ownership boundary. The adapter receives
an accepted stream, reads one chunk with `net::read_chunk`, routes the ordinary
`StreamInput` through a channel, translates `WaitCancelled` into a cleanup
response action, and projects only `SendBytes` actions back to ordered
`net::write_chunk` calls. The matching
`../../examples/specification/check/socket-stream-adapter-cancellable-lifecycle-effects/`
case pins the composed `net`, `time`, and `concurrency` effect boundary while
the handler stays free of transport effects.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-cancellable-deadline-lifecycle/`
covers the cancellable deadline-aware accepted-stream ownership boundary. The
adapter accepts with `net::accept_until_cancellable`, owns the accepted
`NetStream`, reads through `net::read_chunk_until_cancellable`, routes ordinary
`StreamInput` values through a channel, translates accept and read clean-end,
deadline, and cancellation outcomes into adapter decisions or response action
values, and projects only `SendBytes` actions back to ordered
`net::write_chunk` calls. The matching
`../../examples/specification/check/socket-stream-adapter-cancellable-deadline-lifecycle-effects/`
case pins the composed `net`, `time`, and `concurrency` effect boundary while
the handler stays free of transport effects.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-cancel-close-lifecycle/`
covers cancellation cleanup followed by explicit stream close. The adapter
turns `WaitCancelled` into an ordinary cleanup response action, applies only
`SendBytes` actions to `net::write_chunk`, and then calls
`net::close_stream`. The run still passes and records a close event rather than
treating cancellation as a runtime failure.

The executable specification case
`../../examples/specification/run/socket-stream-adapter-clean-shutdown/`
covers adapter-owned clean shutdown after cancellation and deadline-expiry
decisions. The adapter accepts with `net::accept_until_cancellable`, routes an
ordinary `StreamInput` through an existing channel, keeps the handler free of
transport handles, translates cancellation and deadline expiry into ordinary
response actions, applies only `SendBytes` actions as ordered
`net::write_chunk` calls, and then records `net::close_stream` followed by
`net::close_listener`. The matching
`../../examples/specification/check/socket-stream-adapter-clean-shutdown-effects/`
case pins the composed `net`, `time`, and `concurrency` effect boundary while
the handler remains callable without transport effects.

The executable specification cases
`../../examples/specification/run/channel-first-stream-routing-general-list/`,
`../../examples/specification/run/channel-first-stream-routing/`,
`../../examples/specification/run/channel-first-stream-routing-three-route/`,
`../../examples/specification/run/channel-first-stream-routing-four-route/`,
`../../examples/specification/run/channel-select-many-timeout/`,
`../../examples/specification/run/channel-select-timeout-cancellable/`,
`../../examples/specification/run/channel-select-many-timeout-cancellable/`,
`../../examples/specification/run/channel-select-many-timeout-cancellable-forced-cancel/`,
and
`../../examples/specification/run/stream-adapter-cancellable-channel-first-routing/`
cover channel-first selection between ordinary `StreamInput` routes before
handler invocation. They use existing typed channels and
`channel::select_priority`, `channel::select_many_priority`, or
`channel::select_many_timeout`, then call a plain stream handler with explicit
per-stream state. The general receiver-list helper case uses more than four
routes and checks that selected indexes and routed values remain stable. The
timeout case also pins ready receiver-list selection, `None` when no supplied
receiver becomes ready before the timeout, and the matching
`channel::select_many_timeout_result` `Ok(Some(...))` and `Ok(None)` result
boundary. The two-receiver cancellable timeout case pins
`channel::select_timeout_cancellable` ready selection, timeout, and
`Err(SelectError)` cancellation paths. The receiver-list cancellable timeout
cases pin the matching `channel::select_many_timeout_cancellable`
`Ok(Some(...))`, `Ok(None)`, and `Err(SelectError)` paths with
source-visible `CancelToken` observation. The matching
`../../examples/specification/check/channel-first-stream-routing-effects/`,
`../../examples/specification/check/channel-first-stream-routing-general-list-effects/`,
`../../examples/specification/check/channel-first-stream-routing-three-route-effects/`,
`../../examples/specification/check/channel-first-stream-routing-four-route-effects/`,
`../../examples/specification/check/channel-select-many-timeout-effects/`,
`../../examples/specification/check/channel-select-timeout-cancellable-effects/`,
`../../examples/specification/check/channel-select-many-timeout-cancellable-effects/`,
and
`../../examples/specification/check/stream-adapter-cancellable-channel-first-routing-effects/`
cases pin the effect boundary: the routing adapter requires `concurrency`,
the cancellable channel-first adapter requires both `time` and `concurrency`,
socket wrappers around the routing boundary require both `net` and
`concurrency`, and the handler boundary remains free of transport effects.
Earlier bounded route-count examples remain checked coverage, not a pattern
for adding more same-shaped fixtures.

## Pending Input Byte Chunks

The executable specification case
`../../examples/specification/run/pending-input-byte-chunks/` covers the
source-visible pending-input and outgoing-byte chunk slice used by protocol
examples. The example appends `StreamInput.Chunk` byte chunks into a bounded
pending buffer, treats `StreamInput.End` as a distinct event, takes a bounded
`ByteView` while reporting the absolute base `ByteOffset`, drops consumed
bytes while advancing the next absolute offset, reports a retained-input
size-limit failure, materializes the consumed view into an owned `ByteChunk`
that remains readable after the retained pending input advances, and collects
outgoing immutable `ByteChunk` values from ordinary protocol action values
without socket calls.

## HTTP/2 Protocol Core Example

The executable specification case
`../../examples/specification/run/http2-protocol-core/` shows the implemented
ordinary-source HTTP/2 sans-I/O decode-state slice. The example models input
chunks and end-of-stream as explicit ADT events, stores parser state as the
undecoded `ByteChunk` suffix plus the next absolute byte offset, validates the
HTTP/2 client connection preface before any frame header is decoded, and
reuses the generated `Http2FrameHeaderWire` binary schema helper for each
available header after the preface is consumed.

The case pins a valid preface followed by a SETTINGS frame, partial preface
input that waits for more bytes, end-of-stream with a partial preface, a
mismatched preface byte, valid frame arrival after the preface gate,
frame bytes, continuation state after HEADERS retaining opaque header-block
bytes, continuation state across multiple non-final CONTINUATION frames
appending those bytes, completion after a final CONTINUATION with combined
header-block hex/count output, single-frame HEADERS completion when
END_HEADERS is set alongside `END_STREAM`, closed-by-peer stream lifecycle
after accepted HEADERS `END_STREAM` completion through both single-frame
HEADERS and final CONTINUATION paths, continuation ordering failures for a
different frame kind and a different stream id with inspected frame-header
bytes retained for diagnostics, closed input while a header block remains
pending with pending bytes retained for diagnostics, an
accepted unknown extension frame after the client preface gate that preserves
flags, stream id, and bounded payload bytes in an ordinary `UnknownFrame`
value, with the preserved payload also pinned as complete lowercase hex output,
an unknown frame rejected by active continuation state, and an incoming frame
whose payload length exceeds the active receive maximum frame size, plus stream
id domain failures for zero, even, and connection-only stream ids, including
HEADERS and CONTINUATION on the connection stream with and without active
header-block continuation state, and a DATA frame kind rejected for idle-stream
state, plus peer-sent `PUSH_PROMISE` rejected as a known frame kind instead
of preserved as an unknown extension frame and `PUSH_PROMISE` on the
connection stream rejected by the existing stream id domain route. It also
pins zero-length SETTINGS ACK on the connection stream, a valid SETTINGS ACK
clearing outstanding local SETTINGS state, an unexpected SETTINGS ACK with no
outstanding local SETTINGS as `http2.protocol.unexpected_settings_ack` with a
bounded inspected frame-header byte preview,
wrong-length SETTINGS ACK as a typed payload-length failure, SETTINGS ACK on a
nonzero stream as a stream id domain failure, PING frames with and without ACK,
wrong-length PING failures with inspected-payload byte previews, a PRIORITY
frame that exposes dependency stream id, exclusive flag, and weight facts in
the frame value and tracked open-stream state, replacement of those tracked
facts by a later PRIORITY frame for the same stream, a PRIORITY frame on an
idle client-initiated stream that exposes the same priority facts without
opening a peer-created stream, PRIORITY stream-state failures for
closed-by-peer, reset, and mismatched streams, PRIORITY stream id zero,
wrong-length, and self-dependency failures including the idle-stream case, a
GOAWAY frame that moves the connection into
graceful shutdown with last-stream-id and error-code facts, wrong-length
GOAWAY failures, and `RST_STREAM` receive behavior for open, zero-id,
wrong-length, idle-stream, and reset-then-stream-frame cases.
Pending continuation state records the owning stream, starting frame kind,
starting byte offset, and accumulated opaque header-block bytes, and the
closed-input continuation failure projects that context into the stable output.
Receive-limit state records the active maximum frame size with
protocol-default, local-configuration, or local-SETTINGS provenance.
Receive flow-control state records connection receive-window credit and the
tracked open stream receive-window credit. The checked fixture boundary admits
the first idle peer-created HEADERS stream and rejects a second peer-created
HEADERS stream while the first remains open through
`http2.peer_limit.concurrent_streams_exceeded`. DATA consumes the shared
connection window and the targeted stream's own window by payload length.
PADDED DATA consumes
receive-window credit for the full DATA payload, including the pad-length byte
and padding bytes, while the exposed DATA content contains only application
data bytes. A pad length that exceeds the remaining DATA payload is reported as
`http2.protocol.invalid_data_padding`. Accepted DATA with `END_STREAM`, and
accepted HEADERS sequences with `END_STREAM` after header-block completion,
move the tracked stream to closed-by-peer state. Later DATA or stream-level
`WINDOW_UPDATE` for that stream uses the same stream-state failure shape as
other non-open stream states. `WINDOW_UPDATE` on the connection stream
increases connection receive-window credit, and `WINDOW_UPDATE` on an open
stream increases that stream's receive-window credit. A received
`SETTINGS_INITIAL_WINDOW_SIZE` item applies the delta from the previous active
peer setting to the tracked open stream's receive-window credit; adjusted
credit can become negative, and DATA remains blocked on the targeted stream
until `WINDOW_UPDATE` restores enough credit for that stream. A flow-control
rejection does not borrow credit from any unrelated stream state.
Wrong-length `WINDOW_UPDATE` payloads remain typed payload-length failures,
idle-stream `WINDOW_UPDATE` remains the existing stream-state frame-kind
failure, zero increments remain typed protocol failures with inspected payload
byte previews, and overflowing increments remain typed peer-limit failures
without changing window state. DATA payloads larger than the available targeted
stream or connection receive-window credit also remain typed peer-limit
failures. `RST_STREAM` on the open stream decodes its
four-byte error code into reset state, clears the open stream, and leaves
later DATA or stream-level `WINDOW_UPDATE` for that reset stream on the
existing invalid frame-kind path.
Peer-received `SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_FRAME_SIZE`,
`SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_HEADER_TABLE_SIZE`, and `SETTINGS_MAX_HEADER_LIST_SIZE` are stored
as peer-advertised state for outbound decisions with item byte offsets. The
peer-advertised maximum frame size does not replace the inbound receive
maximum used by later frame-size checks, the peer-advertised maximum
concurrent streams value does not replace the local concurrent-stream receive
limit, the peer-advertised maximum header-list size does not replace the
local header-list receive policy used by completed HEADERS or CONTINUATION
checks, and the peer-advertised initial window size does not become an inbound
frame-size or receive-limit provenance entry. Unknown SETTINGS identifiers
leave peer-advertised state unchanged and do not report SETTINGS range
failures; when a later item in the same frame is known, that known item is
still applied or rejected at its own byte offset. Received values for settings
with protocol range constraints are checked before updating peer-advertised
state or open-stream receive-window credit; out-of-range values stay as typed
peer-limit failures at the offending SETTINGS item byte offset. The focused
human and JSON SETTINGS value cases construct a bounded `ByteView` over the
offending six-byte SETTINGS item and pin the related byte preview note plus
the structured JSON preview fields. SETTINGS ACK
frames do not update peer-advertised state or receive-window credit. A valid
SETTINGS ACK clears outstanding local SETTINGS state; an ACK with no
outstanding local SETTINGS is a typed protocol failure. A
final CONTINUATION with END_HEADERS clears continuation state and exposes the
completed accumulated header-block bytes in observable example output.
The same HPACK fixture boundary accepts the static indexed `0x81`
`:authority` with an empty value, `0x82` `:method: GET`, `0x83`
`:method: POST`, `0x84` `:path: /`, `0x85` `:path: /index.html`, `0x86`
`:scheme: http`, `0x87` `:scheme: https`,
`0x88` `:status: 200`, `0x89` `:status: 204`, `0x8a` `:status: 206`,
`0x8b` `:status: 304`, `0x8c` `:status: 400`, `0x8d` `:status: 404`, and
`0x8e` `:status: 500`, plus `0x8f` `accept-charset:`,
`0x90` `accept-encoding: gzip, deflate`, `0x91` `accept-language:`,
`0x92` `accept-ranges:`, `0x93` `accept:`, `0x94`
`access-control-allow-origin:`, `0x95` `age:`, `0x96` `allow:`, `0x97`
`authorization:`, `0x98` `cache-control:`, `0x99`
`content-disposition:`, `0x9a` `content-encoding:`, `0x9b`
`content-language:`, `0x9c` `content-length:`, `0x9d`
	`content-location:`, `0x9e` `content-range:`, `0x9f`
	`content-type:`, `0xa0` `cookie:`, `0xa1` `date:`, `0xa2` `etag:`,
	`0xa3` `expect:`, `0xa4` `expires:`, `0xa5` `from:`, `0xa6`
	`host:`, `0xa7` `if-match:`, `0xa8` `if-modified-since:`, `0xa9`
	`if-none-match:`, `0xaa` `if-range:`, `0xab`
	`if-unmodified-since:`, `0xac` `last-modified:`, `0xad` `link:`,
	`0xae` `location:`, `0xaf` `max-forwards:`, `0xb0`
	`proxy-authenticate:`, `0xb1` `proxy-authorization:`, `0xb2` `range:`,
	`0xb3` `referer:`, `0xb4` `refresh:`, `0xb5` `retry-after:`, `0xb6`
	`server:`, `0xb7` `set-cookie:`, `0xb8`
	`strict-transport-security:`, `0xb9` `transfer-encoding:`, `0xba`
	`user-agent:`, `0xbb` `vary:`, `0xbc` `via:`, and `0xbd`
	`www-authenticate:`
	header-block bytes. It also checks `0x82 0x84` as a two-header static
	indexed block that preserves `:method: GET` followed by `:path: /` in
	the source-visible header list. The HTTP/2 protocol-core example also
	carries static indexed `0x85` `:path: /index.html` through a completed
	final CONTINUATION frame before HPACK decode. It also checks
	literal-without-indexing,
	literal-with-indexing,
	and literal-never-indexed fixtures whose indexed-name form names a
supported static-table header name already accepted by the static-indexed
fixture set, including ordinary names such as `server`, `content-type`, and
`user-agent`. Those literal fixtures share the HPACK string literal decoder
for visible-ASCII raw values and Huffman-marked values decoded by scanning
the HPACK static Huffman table across the full byte symbol range rather than a
fixed decoded-value allowlist. The checked Huffman fixture boundary accepts
visible ASCII, the line-feed fixture value, and single-byte `hpack-byte-xx`
labels for every byte value; multi-byte decoded non-visible byte strings remain
unsupported fixture inputs. The fixture
also accepts raw new-name literal forms whose field-name string is a raw
visible-ASCII HPACK string literal, and sends those decoded names through the
same HTTP/2 header-list validation paths as indexed-name literals. The same
decoder accepts checked
one-continuation string-length prefixes for long raw and Huffman-marked values
through all three literal forms, including a 129-byte raw `:authority` value
past the former checked raw decode boundary. The same HPACK-prefixed integer
foundation reads checked table-size updates, dynamic-name indexes, and string
literal lengths before each caller applies its fixture-specific policy. The
HTTP/2 protocol-core example
uses the same static Huffman table to encode checked Huffman-marked outbound
fixture string literals, including a non-allowlist
`:authority: abc.test` value whose
checked bytes are `0x01 0x86 0x1c 0x64 0x5d 0x25 0x42 0x7f`, a line-feed
`:path` value whose checked bytes are
`0x04 0x84 0xff 0xff 0xff 0xf3`, and a single-NUL `:path` value whose checked
bytes are `0x04 0x82 0xff 0xc7`, plus `hpack-byte-ff` whose checked bytes are
`0x04 0x84 0xff 0xff 0xfb 0xbf`. It keeps a multi-byte Huffman-marked
non-visible value on the raw string encoding failure path. The
boundary example also checks
`encode_hpack_raw_string_literal` for a short raw `PUT` literal that keeps its
existing bytes, a visible ASCII `bad` literal that was not part of the former
fixture allowlist, a long raw `a` literal that uses the same
one-continuation HPACK integer length boundary, and a non-visible raw byte
value that remains on the fixture-owned unsupported-header-block failure path
with expected fixture `fixture raw string encoding`. The example
covers additional non-allowlist visible-ASCII raw values through
literal-without-indexing `:authority: odd`,
literal-with-indexing `:method: raw`, and literal-never-indexed
`:path: bot`, plus `:authority: abc.test` through completed HEADERS and final
CONTINUATION paths, raw `:status` through completed HEADERS, Huffman
`:path: test`, `:path` line feed, and `:path` single NUL through completed
HEADERS, Huffman `:path` `hpack-byte-ff` through completed HEADERS, Huffman
`:method: PUT` through both
literal-without-indexing and literal-with-indexing, Huffman `:method: bad`
through literal-without-indexing, literal-with-indexing, and
literal-never-indexed, Huffman `:status: 200` through completed HEADERS and
final CONTINUATION, raw literal-with-indexing `:authority`, Huffman
literal-with-indexing `:scheme: https`, raw literal-with-indexing `:status`,
raw literal-never-indexed `:path` through completed HEADERS and final
CONTINUATION, and long raw and Huffman-marked string-length continuation
fixtures. It also covers ordinary static-name literals: raw
literal-without-indexing `server: ok` as `0x0f 0x27 0x02 "ok"`, raw
literal-with-indexing `content-type: text` as `0x5f 0x04 "text"` followed
by a later `0xbe` dynamic-indexed reuse, and raw literal-never-indexed
`user-agent: agent` through a final CONTINUATION as
`0x1f 0x2b 0x05 "agent"`. It also covers raw new-name
literal-with-indexing `x-trace: ok` followed by dynamic-indexed reuse, plus
raw new-name trailers that accept lower-case `x-trace` and reject uppercase
`Server` and token-invalid `bad@name` through the existing trailer diagnostics.
Focused human and JSON examples pin those raw field-name failures on the same
`http2.protocol.invalid_request_header_list` projection as indexed-name
header-list failures.
Completed HEADERS and final CONTINUATION paths reach
the long-value HPACK boundary before the protocol-core header-list receive
limit rejects the decoded size. Malformed string length including
non-terminating string-length continuations has
`hpack.fixture.malformed_string_length`. Malformed raw string values for
supported literal names, including non-visible raw bytes and malformed raw
`:status` literals, have `hpack.fixture.malformed_raw_string_value`.
Malformed Huffman padding has `hpack.fixture.malformed_huffman_padding`.
Huffman EOS used as a decoded symbol has `hpack.fixture.huffman_eos_symbol`,
and multi-byte non-visible Huffman strings outside the supported checked
single-byte labels have `hpack.fixture.huffman_non_visible_value`. The focused failures
are checked through completed HEADERS or final CONTINUATION paths as
appropriate.
It does not implement general HPACK behavior beyond the fixture boundary.
Checked HEADERS bytes
include zero-length `:path` as `0x04 0x80`, `:path: test` as
`0x04 0x83 0x49 0x50 0x9f`, `:scheme: https` as
`0x06 0x84 0x9d 0x29 0xad 0x1f`, `:status: 200` as
`0x08 0x82 0x10 0x01`, `:method: bad` as
`0x02 0x83 0x8c 0x72 0x7f`, `0x42 0x83 0x8c 0x72 0x7f`, and
`0x12 0x83 0x8c 0x72 0x7f`, and `:authority: www.example.com` as
`0x01 0x8c 0xf1 0xe3 0xc2 0xe5 0xf2 0x3a 0x6b 0xa0 0xab 0x90 0xf4 0xff`.
The focused HPACK boundary also checks raw literal-never-indexed
`:authority: abc.test` as `0x11 0x08 "abc.test"`, Huffman-marked
literal-never-indexed `:scheme: https` as
`0x16 0x84 0x9d 0x29 0xad 0x1f`, and long raw and Huffman-marked
literal-never-indexed string-length boundaries. The protocol-core example
also mirrors a 129-byte raw literal through the final CONTINUATION path before
the local header-list receive limit rejects the decoded size.
The
source-level HPACK
boundary also checks one dynamic-table receive slice: a literal
incremental-indexing `:path: /target` block returns a next immutable fixture
state that the HTTP/2 decode state carries, a later `0xbe` indexed header
field decodes through that carried state, and the same indexed byte without a
dynamic entry stays unsupported. Later literal incremental-indexing blocks
prepend newest-first bounded fixture dynamic-table entries while older entries
remain addressable when the table has room. After `:method: PUT` and
`:scheme: https` are inserted over `:path: /target`, `0xbe` decodes the
newest `:scheme: https` entry, `0xbf` decodes the second `:method: PUT`
entry, and `0xc0` decodes the third retained `:path: /target` entry.
Completed HEADERS and final CONTINUATION paths both carry that HPACK state
before later header blocks are decoded. The HTTP/2 example also prints the
fixture decode count before and after a split header block, showing that a
pending CONTINUATION frame leaves HPACK state unchanged until the final
accepted header-block decode. The checked dynamic-name
literal-with-indexing form `0x7e 0x06 "/again"` reuses the newest dynamic
entry name `:path`, inserts `:path: /again` as the newest entry, and leaves
the older `:path: /target` entry readable while the bounded fixture table has
room. With three retained dynamic entries, the boundary also checks
continuation-byte indexed-name values `63` and `64` through
`0x7f 0x00 0x05 "PATCH"` and `0x7f 0x01 0x06 "/third"`, then reads the
inserted newest entries through the carried fixture state. A final
CONTINUATION path covers the value `63` form before a later header block reads
the inserted `:method: PATCH` entry. The boundary also checks a deeper
bounded dynamic table where dynamic index value `127` is encoded as
`0x7f 0x40 0x05 "/deep"`, reuses an older retained `:path` name, inserts
`:path: /deep`, and carries that insertion through both completed HEADERS and
final CONTINUATION paths before later `0xbe` reads.
Literal-without-indexing and literal-never-indexed dynamic-name forms reuse
the same dynamic-table name lookup without inserting replacement dynamic
entries; the focused HPACK
boundary checks `0x0f 0x2f 0x03 "/no"` and
`0x1f 0x2f 0x07 "/secret"` after `:path: /target` has been inserted, then
reads the retained `:path: /target` entry through `0xbe` from each returned
state. After `:method: PUT` has also been inserted, the HTTP/2 protocol-core
case checks the one-continuation indexed-name forms `0x0f 0x30 0x03 "/no"`
and `0x1f 0x30 0x07 "/secret"` for dynamic index `63`; both reuse
`:path`, decode the visible-ASCII values, advance the fixture decode count,
and leave later `0xbe` and `0xbf` reads pointed at the prior `:method: PUT`
and `:path: /target` entries. The HTTP/2 protocol-core case also covers
dynamic index value `127` for those two non-inserting forms with
`0x0f 0x70 0x05 "/skip"` and `0x1f 0x70 0x07 "/secret"`, then proves a later
`0xff` read still observes the older retained `:path: /a` entry. A
literal-never-indexed decode without a prior
dynamic entry still
advances the immutable fixture decode count without inserting a dynamic-table
entry, so a following `0xbe` dynamic-indexed lookup from that returned state
remains unsupported. The case also checks a `0xbe` dynamic-indexed lookup
without any prior dynamic entry at the HTTP/2 boundary: the unsupported
fixture failure leaves the carried decode count unchanged, and a later
accepted literal-with-indexing block inserts `:path: /target` so the following
`0xbe` reads through the returned state. Missing, malformed, and out-of-range
dynamic-name continuations remain unsupported. The fixture also accepts dynamic
table-size update bytes `0x3e`, `0x3f`, `0x3f 0x01`, `0x3f 0x0b`,
`0x3f 0x80 0x01`, `0x3f 0x81 0x01`, and `0x3f 0x82 0x02`, exposes the
resulting checked table
sizes `30`, `31`, `32`, `42`, `159`, `160`, and `289` through the fixture-state
accessor, and the HTTP/2 example covers both completed HEADERS blocks and
final CONTINUATION blocks carrying accepted updated immutable states into
later header block decodes or rejecting updates above the local header-table
receive policy, including `0x3f 0xe1 0x1f` for the current fixture table size.
If a complete dynamic table-size update appears after an already decoded
header field in the same completed header block, the HTTP/2 example rejects
it on both completed HEADERS and final CONTINUATION paths with
`hpack.fixture.table_size_update_not_at_start`; the diagnostic reports the
update byte offset, requested table size, frame kind, stream id, active HPACK
fixture state, codec module, expected fixture boundary, and bounded preview
bytes.
Malformed non-terminating table-size updates and table-size updates with
trailing bytes after a complete integer remain unsupported.
Reducing the fixture table size below the supported entries
uses the accepted header name byte count plus value byte count plus `32` for
each dynamic entry and evicts oldest entries first: table size `86` retains
the newest `:scheme: https` and second `:method: PUT` entries while evicting
the third `:path: /target` entry; table size `42` retains the newest
`:method: PUT` entry when that entry is followed by `:path: /target`, evicts
the older `:path: /target` entry, and also evicts `:authority: abc.test`;
table size `40` evicts the raw new-name ordinary `x-trace: ok` entry after a
checked `0xbe` reuse; table size `30` evicts both supported `:method: PUT` and
`:path: /target` dynamic
entries and leaves later dynamic indexed representations on the unsupported
fixture path. A later literal-with-indexing insertion that exceeds remaining
capacity keeps the inserted `:path: /target` entry readable at `0xbe` and
evicts the older entries so `0xbf` stays unsupported. The fixture
exposes the decoded header name and value through ordinary header-list
accessors, advances the immutable fixture state, and keeps unsupported HPACK
input on `hpack.fixture.unsupported_header_block`, including unsupported
literal-without-indexing forms. Malformed string lengths, malformed raw string
values for supported literal names, malformed Huffman padding, Huffman EOS,
and multi-byte non-visible Huffman strings outside the supported checked
single-byte labels stay on the
HPACK fixture boundary but use focused `hpack.fixture.*` ids, and the focused
JSON and human examples assert their bounded header-block byte previews.
The table-size placement diagnostic has matching focused human and
`run --json` examples.
The outbound DATA send-intent slice keeps outbound connection and stream
credit separate from inbound receive windows. It accepts a DATA intent whose
full payload fits available outbound connection and stream windows, including
the boundary where either available window is exactly consumed and the other
window still has credit. Payloads larger than the peer-advertised maximum
frame size are emitted in one
immutable output chunk containing multiple DATA frames, each no larger than
that maximum, then both outbound credits are consumed by the full encoded
DATA payload length after all frames encode. `END_STREAM` appears only on the
final DATA frame when requested. PADDED DATA send-intents encode the PADDED
flag, one pad-length byte per emitted frame, application bytes, and requested
zero padding bytes. The peer-advertised maximum frame size and outbound
connection and stream credit count the pad-length byte and padding as part of
each encoded DATA payload. Padding that cannot fit in the selected frame
payload, DATA intents that exceed available outbound connection credit, and
DATA intents that exceed peer-advertised stream credit derived from received
`SETTINGS_INITIAL_WINDOW_SIZE` are rejected before output bytes are emitted.
The checked case also pins zero available connection credit and zero available
stream credit as stable rejected no-output outcomes.
After receiving GOAWAY or after locally sending GOAWAY, the outbound DATA
send-intent slice accepts DATA at the recorded last-stream-id boundary and
rejects a higher open stream with `http2.protocol.stream_after_goaway` before
frame-size splitting, encode checks, or outbound credit changes. Missing,
closed, reset, and mismatched stream cases keep their narrower existing
failures, and rejected post-GOAWAY DATA emits no output chunk.
Accepted DATA with `END_STREAM` records local closed-stream state; later
outbound DATA, outbound HEADERS, and stream-level outbound `WINDOW_UPDATE` for
that stream use the same closed stream-state rejection boundary. The receive
core records that local `END_STREAM` as half-closed-local for inbound
processing: inbound DATA on that stream still consumes connection and stream
receive-window credit, PADDED DATA still exposes only application content,
invalid padding and window-credit failures stay typed to the half-closed-local
state, and inbound DATA with peer `END_STREAM` moves the stream to the
closed-by-peer state. Generated DATA frame-header representation failures
remain codec encode errors.
The local SETTINGS send-intent slice emits supported local SETTINGS items for
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
`SETTINGS_MAX_FRAME_SIZE`, and `SETTINGS_MAX_HEADER_LIST_SIZE`. Accepted
single-item and two-item batch intents emit one frame-header-plus-payload
chunk with length `6 * item_count`, kind `4`, flags `0`, stream id `0`, and
the selected setting identifier and four-byte unsigned value pairs in order,
then record one outstanding local SETTINGS batch with the selected item
count. Local `SETTINGS_ENABLE_PUSH` values outside `0..1` are rejected before
bytes are emitted with the SETTINGS range failure shape, including when the
invalid value appears in a batch. A valid SETTINGS ACK clears that outstanding
state, including a multi-item batch, and an ACK with no outstanding local
SETTINGS stays on the typed unexpected-ACK failure path.
The outbound PING ACK send-intent slice accepts a valid inbound non-ACK PING,
emits one frame-header plus opaque-payload output chunk with length `8`, kind
`6`, ACK flag `1`, and stream id `0`, and preserves the original eight-byte
PING payload. A received PING ACK remains visible as a received ACK frame and
emits an empty output chunk list.
The outbound `RST_STREAM` send-intent slice accepts a nonzero currently open
stream, emits a frame-header plus error-code output chunk with length `4`,
kind `3`, flags `0`, and the selected stream id, then records local reset
state so a later stream-level `WINDOW_UPDATE` for that stream follows the
same reset stream-state rejection boundary. It rejects stream id `0`, missing
streams, closed streams, already reset streams, mismatched open streams, and
generated encode-helper representation failures for the stream id or
error-code payload before accepted bytes are produced.
The outbound PRIORITY send-intent slice accepts a nonzero currently open
stream and emits one frame-header plus priority payload output chunk with
length `5`, kind `2`, flags `0`, the selected stream id, a dependency payload
whose high bit carries the exclusive flag, and the selected weight. The slice
pins replacement-friendly dependency id, exclusive flag, and weight values,
rejects stream id `0`, missing streams, closed streams, already reset streams,
mismatched open streams, and self-dependency before output bytes are produced,
and preserves generated encode-helper representation failures for the frame
stream id or dependency payload as `codec.encode_value_unrepresentable`
encode errors.
The outbound HEADERS send-intent slice accepts an already-encoded opaque
header-block chunk for a nonzero currently open stream. Header blocks within
the peer-advertised maximum frame size emit one HEADERS frame-header plus
payload output chunk with kind `1`, `END_HEADERS`, and optional `END_STREAM`.
Larger header blocks emit one output chunk containing a HEADERS frame followed
by one or more CONTINUATION frames on the same stream. Each emitted frame
payload respects the peer-advertised maximum frame size, `END_HEADERS` appears
only on the final frame, and optional `END_STREAM` stays on the first HEADERS
frame. Accepted `END_STREAM` records local closed-stream state. The checked
case pins one-continuation, multiple-continuation, and `END_STREAM`-plus-final
`END_HEADERS` split outputs as complete lowercase hex. The same checked case
pins fixture-encoded Huffman-marked `:path: test` and
`:authority: abc.test` header blocks inside outbound HEADERS frames. It
rejects stream id `0`, missing streams, closed
streams, already reset streams, mismatched open streams, and generated
frame-header representation failures before accepted bytes are produced.
After receiving GOAWAY or after locally sending GOAWAY, the same slice accepts
outbound HEADERS at the recorded last-stream-id boundary and rejects a higher
open stream through the existing `http2.protocol.stream_after_goaway`
diagnostic before frame splitting or encode checks. The checked output keeps
the endpoint role visible for the local-GOAWAY rejection. Stream id zero and
closed stream cases keep their narrower existing failures.
The outbound `PUSH_PROMISE` send-intent slice accepts a currently open
client-created associated stream, a server-initiated promised stream id, and
already-encoded opaque header-block bytes. It pins a single-frame
`PUSH_PROMISE` output and a split output where the first frame carries the
generated promised-stream payload plus the first header-block fragment and the
final CONTINUATION frame carries `END_HEADERS`. The checked case also pins a
fixture-encoded Huffman-marked `:status: 200` header block inside an outbound
`PUSH_PROMISE` frame. It rejects stream id `0`, missing, closed, reset,
mismatched, or server-created associated streams, promised stream id `0`, and
representable client-initiated promised stream ids before accepted bytes are
produced, while preserving out-of-range promised stream ids as generated
payload encode errors.
The outbound GOAWAY send-intent slice accepts a last stream id and error code,
emits a frame-header plus GOAWAY payload output chunk with length `8`, kind
`7`, flags `0`, and stream id `0`, then records local graceful-shutdown state
so a later peer-created HEADERS stream greater than the sent last stream id
follows the existing post-GOAWAY stream rejection boundary. It preserves
generated schema payload encode-helper representation failures for the last
stream id or error-code payload before accepted bytes are produced. The
focused `run/binary-schema-goaway-payload-encode/` case pins the same
`ReservedBits(1, 0)` plus `UInt31be` and `UInt32be` payload boundary through
the general `byte_encode_<schema>` helper path.
Protocol failures stay as ordinary ADT values and are projected by source code
into stable diagnostic ids and related context fields for byte offset,
observed and allowed lengths, actual and expected frame kind, stream reference,
active continuation, connection state, or stream state, setting identity, accepted
SETTINGS range, receive-limit provenance, peer-limit provenance, payload length
expectations, required stream id domain, endpoint role, matched preface prefix
count, expected and actual preface byte, and rule provenance.
The same case also pins outbound frame header encoding from an ordinary
record-shaped frame description through the generated binary schema encode
helper. The checked `[[output_chunk_list]]` fixtures cover a SETTINGS header
on the connection stream, a DATA header on a nonzero stream, local SETTINGS
frame-header-plus-item chunks for `SETTINGS_HEADER_TABLE_SIZE`,
`SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_ENABLE_PUSH`,
`SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_MAX_FRAME_SIZE`, and
`SETTINGS_MAX_HEADER_LIST_SIZE`, an accepted `RST_STREAM` frame plus error-code
payload, accepted PRIORITY frame-header-plus-priority-payload chunks,
accepted HEADERS frame-header-plus-header-block chunks with and without
`END_STREAM`, an accepted post-GOAWAY HEADERS frame at the recorded boundary,
an accepted post-local-GOAWAY HEADERS frame at the recorded boundary,
accepted post-GOAWAY and post-local-GOAWAY DATA frames at the recorded
boundary, empty chunk lists for above-boundary post-GOAWAY DATA rejections,
accepted outbound HPACK dynamic table-size update HEADERS chunks for the
one-byte and saturated-prefix continuation integer forms, a later HEADERS
chunk that observes the reduced outbound HPACK table capacity, an empty chunk
list for an outbound table-size update rejected above the peer-advertised
`SETTINGS_HEADER_TABLE_SIZE`,
accepted `PUSH_PROMISE` frame-header-plus-promised-stream-payload chunks,
an accepted GOAWAY frame plus last-stream-id and error-code payload, and the
maximum valid `UInt31be` stream id. The source
output also matches generated helper `codec.encode_value_unrepresentable`
failure for an out-of-range stream id, keeping field path and reason text
visible without converting it into a protocol diagnostic.
The checked HTTP/2 source output also pins outbound `WINDOW_UPDATE`
send-intents for accepted connection-level and open-stream receive-credit
increments, zero and out-of-range increments, current-window overflow,
stream id zero, idle, closed, reset, mismatched, and generated frame-header
and increment-payload representation failure cases. Rejected intents keep
output chunks empty.

`../../examples/specification/run/http2-protocol-core-closed-human/`,
`../../examples/specification/run/http2-protocol-core-closed-json/`,
`../../examples/specification/run/http2-protocol-core-preface-partial-human/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-human/`,
`../../examples/specification/run/http2-protocol-core-continuation-human/`,
`../../examples/specification/run/http2-protocol-core-continuation-json/`,
`../../examples/specification/run/http2-protocol-core-frame-size-human/`,
`../../examples/specification/run/http2-protocol-core-settings-value-human/`,
`../../examples/specification/run/http2-protocol-core-window-update-increment-human/`,
`../../examples/specification/run/http2-protocol-core-flow-control-human/`,
`../../examples/specification/run/http2-protocol-core-data-padding-human/`,
`../../examples/specification/run/http2-protocol-core-hpack-string-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-raw-string-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-raw-name-token-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-table-size-placement-human/case.toml`,
`../../examples/specification/run/hpack-fixture-huffman-eos-human/`,
`../../examples/specification/run/hpack-fixture-huffman-non-visible-human/`,
`../../examples/specification/run/http2-protocol-core-request-headers-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-order-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-token-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-scheme-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-content-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-response-headers-content-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-concurrent-streams-human/`,
`../../examples/specification/run/http2-protocol-core-invalid-stream-id-human/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-push-promise-human/`,
`../../examples/specification/run/http2-protocol-core-local-stream-after-goaway-human/`,
`../../examples/specification/run/http2-protocol-core-settings-ack-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-settings-unexpected-ack-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-ping-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-priority-dependency-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-goaway-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-frame-size-json/`,
`../../examples/specification/run/http2-protocol-core-preface-partial-json/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-json/`,
`../../examples/specification/run/http2-protocol-core-settings-value-json/`,
`../../examples/specification/run/http2-protocol-core-window-update-increment-json/`,
`../../examples/specification/run/http2-protocol-core-flow-control-json/`,
`../../examples/specification/run/http2-protocol-core-data-padding-json/`,
`../../examples/specification/run/http2-protocol-core-hpack-string-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-raw-string-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-raw-name-uppercase-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-hpack-table-size-placement-json/case.toml`,
`../../examples/specification/run/hpack-fixture-huffman-eos-json/`,
`../../examples/specification/run/hpack-fixture-huffman-non-visible-json/`,
`../../examples/specification/run/http2-protocol-core-request-headers-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-duplicate-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-connection-specific-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-uppercase-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-scheme-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-request-headers-content-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-response-headers-content-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-concurrent-streams-json/`,
`../../examples/specification/run/http2-protocol-core-invalid-stream-id-json/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-stream-state-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-push-promise-json/`,
`../../examples/specification/run/http2-protocol-core-local-stream-after-goaway-json/`,
`../../examples/specification/run/http2-protocol-core-settings-ack-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-settings-unexpected-ack-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-ping-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-priority-dependency-json/case.toml`,
and `../../examples/specification/run/http2-protocol-core-goaway-length-json/case.toml`
pin the command-facing projection path for those typed failures. The human
cases check focused primary messages and related context, while the JSON cases
check `protocol_diagnostic` details for byte offset, frame kind, stream id,
active continuation, connection state, or stream state, observed and allowed
frame sizes, malformed HPACK Huffman padding fixture context, setting identity,
observed setting value, accepted setting range,
stream reference, receive-limit provenance, peer-limit provenance, observed and
expected payload length including SETTINGS ACK length zero and `RST_STREAM`
length four, unexpected SETTINGS ACK state, flow-control window credit,
expected and actual
preface byte values, matched preface prefix count, expected preface byte count,
structured bounded preface, invalid-stream-id frame-header,
invalid-frame-kind, and invalid-payload byte preview fields, concurrent-stream
attempted and allowed counts, required stream id domain, endpoint role,
PRIORITY dependency stream id, structured bounded PRIORITY payload byte
preview fields, and rule provenance. The
request-header projection cases cover missing required request pseudo-headers,
response-only `:status` pseudo-headers, duplicate request pseudo-headers, and
request pseudo-headers after regular headers, uppercase ordinary header names,
ordinary header names outside the HTTP field-name token shape, and invalid
`te` values on inbound requests, plus invalid and mismatched
`content-length` values, with decoded header names carried as related context
or structured JSON details. The
response-header projection cases cover missing and duplicate `:status`,
request-only pseudo-headers, and response pseudo-headers after regular
headers, invalid request `:scheme` values, invalid `te` values, and invalid
and mismatched `content-length` values, with the same JSON detail and human
related-note shape. The larger protocol-core case also checks an accepted
fixture-marked request header list, accepted request `:scheme` values `http`
and `https` through completed HEADERS and final CONTINUATION paths, an
unsupported `:scheme` value, accepted `te: trailers`, a final CONTINUATION
path missing `:method`, a completed HEADERS path containing response-only
`:status`, a duplicate `:method`, and a `:method` after a regular `host`
header, plus uppercase and token-invalid ordinary request header names,
connection-specific ordinary request header names `connection`, `keep-alive`,
`proxy-connection`, `transfer-encoding`, and `upgrade`, and an invalid `te`
value. It accepts one and repeated matching valid decimal
`content-length` request values and rejects mismatched, empty, non-decimal,
signed, whitespace-padded, and negative-looking request values. It also
checks inbound request trailers on an already-open stream: ordinary trailer
fields are accepted through completed HEADERS and final CONTINUATION paths,
accepted trailers close the stream by peer without consuming receive-window
credit, a second HEADERS block without peer `END_STREAM` is rejected as a
request-trailer state failure, pseudo-header trailers are rejected with active
state `request-trailers`, and uppercase ordinary names, invalid
ordinary-name tokens, connection-specific ordinary names, and invalid `te`
values keep the same structured request header-list failure fields.
The same larger case checks accepted fixture-marked response header lists
including a final
CONTINUATION path for `te: trailers`, a final
CONTINUATION path missing `:status`, duplicate `:status`, request-only
`:method` and `:authority`, and `:status` after a regular `server` header,
plus uppercase and token-invalid ordinary response header names and an
invalid `te` value. It accepts one and repeated matching valid decimal
`content-length` response values and rejects mismatched, empty, non-decimal,
signed, whitespace-padded, and negative-looking response values. The focused
frame-kind, stream-id, and `PUSH_PROMISE`
projection examples declare
`Http2FrameHeaderWire` and decode through the generated schema helper before
projecting protocol diagnostics, so those command-facing cases cover the
general schema helper path as well as the larger protocol-core fixture. The
preface and PRIORITY dependency human cases also check nearby-byte notes rendered as
bounded lowercase hex pairs with total byte count and truncation state. The
concurrent-stream command fixtures cover the focused peer-created stream limit
projection, including endpoint-role context, while the ordinary protocol-core
case covers the receive-core rejection when a second peer-created stream would
exceed the active limit. The flow-control command fixtures cover stream
receive-window provenance while the ordinary protocol-core case also covers
connection receive-window provenance and the `WINDOW_UPDATE` receive-credit
slice.
The frame-size command fixtures cover local-configuration provenance while the
ordinary protocol-core case keeps the protocol-default, local-configuration,
local-SETTINGS, peer-advertised SETTINGS, rejected peer-advertised SETTINGS,
and peer-advertised initial-window receive-window distinctions visible in
executable output.
