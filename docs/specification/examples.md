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
projection. The named-fixture truncation case pins the same JSON diagnostic
shape while proving that valid fixture bytes fail as codec truncation, not as
fixture text validation.

## Codec Decode Step Vocabulary

The executable specification case
`../../examples/specification/run/codec-decode-step-vocabulary/` covers the
source-visible incremental decode transition vocabulary. Ordinary source
functions construct `DecodeStep<T>` values for a successful `Decoded` outcome
with a decoded value and consumed `ByteCount`, a `NeedMore` outcome with
`NeedBytes` readiness that consumes no input, and an `Invalid` outcome carrying
a structured `DecodeError` with id, byte offset, and field path.
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
The executable specification case
`../../examples/specification/run/derived-codec-decode-boundary/` covers a
derived codec decode boundary for the same eligible generated binary schema
decode-step slice: a codec item call observes the generated helper's
`Decoded`, `NeedMore`, and `Invalid` `DecodeStep<T>` values through the codec
item name while preserving mapped record fields and no-consumption outcomes.
The executable specification case
`../../examples/specification/run/derived-codec-repeat-decode-boundary/`
covers the same derived codec call boundary when the generated decode-step
helper decodes a bounded repeated primitive field and reports repeat-backed
readiness or helper failure through the codec item.
The executable specification case
`../../examples/specification/run/derived-codec-nested-dispatch-decode-boundary/`
covers the same derived codec call boundary when the generated decode-step
helper decodes a same-module nested dispatch payload schema.
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
The executable specification case
`../../examples/specification/run/derived-codec-encode-boundary/` covers a
derived codec encode boundary for the eligible generated binary schema encode
helper slice: a codec item call observes successful helper output as
`Encoded(List<ByteChunk>)` with one chunk and out-of-range generated helper
failures as `Invalid(EncodeError)`.
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
writes a same-module nested dispatch payload schema and projects dispatch
selection failures as `Invalid(EncodeError)`.
The executable specification case
`../../examples/specification/run/binary-schema-general-helper-roundtrip/`
covers the same derived codec encode boundary over the combined non-HTTP
schema shape listed above and checks that helper `Ok(ByteChunk)` output
projects to one `Encoded(List<ByteChunk>)` chunk, while helper
`Err(EncodeError)` output projects to `Invalid(EncodeError)`.
The derived mapping-boundary diagnostics case listed above pins the matching
`codec.encode_value_type` rejection for generated encode boundaries.

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
`../../examples/specification/run/binary-byteview-u64-truncated-json/`, and
`../../examples/specification/run/binary-byteview-u64-write-failure-human/`
cover the ordinary prelude byte-helper `u64` slice. The runtime cases prove
big-endian and little-endian read byte order, matching write byte order,
truncated-read diagnostics, and the source-visible `Int` write boundary.

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
cover the implemented frame-header primitive decode slice. The valid case
checks `UInt24be`, two `UInt8` fields, `ReservedBits(1, 0)`, and `UInt31be`
over one `ByteView`, and its expected record omits the reserved field. The
failure cases pin structured `schema.truncated_field` and
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
`../../examples/specification/run/binary-schema-mapping-selection-decode/`
pins deterministic mapping selection by an already decoded field value.
`../../examples/specification/check/schema-mapping-selection-diagnostics/`
pins JSON diagnostics for missing, ambiguous, and unsupported mapping
selection.
`../../examples/specification/check/schema-mapping-expression-boundary-diagnostics/`
pins unsupported mapping expression, unresolved constructor, constructor
arity, and constructor payload type diagnostics.

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

`../../examples/specification/run/binary-schema-reserved-bit-encode/` pins the
reserved-bit encode slice for `ReservedBits(1, 0)` followed by `UInt31be`.
The case checks complete lowercase hex output for an HTTP/2-style stream
identifier field and the `UInt31be` maximum boundary. The adjacent checker
case
`../../examples/specification/check/schema-reserved-bit-encode-diagnostics/`
asserts `schema.reserved_bits_encode` for a non-byte-aligned reserved-bit
shape outside the supported encode layouts.
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

`../../examples/specification/run/binary-schema-closed-dispatch-encode/`
pins the closed dispatch encode helper slice. The passing cases select
`UInt8`, `UInt16be`, `UInt24be`, and `UInt32be` payload widths from an earlier
tag field and write one `ByteChunk` in declaration order.
`../../examples/specification/run/binary-schema-closed-dispatch-nested-encode/`
pins same-module nested payload encode for a closed dispatch case.
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`
pins closed and extension-tolerant nested payload encode through the generated
schema helper path, including byte-aligned reserved fields and little-endian
primitive output.
`../../examples/specification/run/binary-schema-imported-closed-dispatch-nested-encode/`
pins public imported nested payload encode for a closed dispatch case.
`../../examples/specification/run/binary-schema-closed-dispatch-encode-unknown-tag/`
asserts `codec.dispatch_unknown_tag` when the tag value has no closed case.
`../../examples/specification/run/binary-schema-closed-dispatch-encode-out-of-range/`
asserts `codec.encode_value_unrepresentable` against the selected `UInt8`
payload case.

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

`../../examples/specification/run/binary-schema-closed-dispatch-decode/`,
`../../examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
`../../examples/specification/run/binary-schema-imported-closed-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-dispatch-nested-failure-json/`,
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-failure-json/`,
`../../examples/specification/run/binary-schema-imported-dispatch-nested-failure-json/`,
`../../examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
and
`../../examples/specification/run/binary-schema-closed-dispatch-unknown-human/`
pin the narrow closed dispatch slice. The passing case decodes a known tag and
selected primitive payload as ordinary `Int` fields; the nested passing cases
decode selected same-module and public imported payload schemas as
record-shaped fields. The general helper passing case proves selected nested
payload schemas keep fixed-field validation, byte-aligned reserved fields, and
little-endian primitive reads when reached through closed or extension
dispatch. The nested failure cases pin the nested schema field path and
absolute byte offset, including fixed-field mismatch diagnostics produced by
the nested helper. The unknown-tag failing cases assert
`schema.dispatch_unknown_tag`, the dispatch byte offset, structured field path,
decoded tag field and value, expected tag values, structured byte preview
fields, and focused human related notes.
`../../examples/specification/run/derived-codec-imported-nested-dispatch-decode-boundary/`
pins the same public imported nested payload helper eligibility when reached
through a `derive decode` codec boundary, including the returned
`DecodeStep` consumed count.
`../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`
pins the static boundary for nested dispatch payload schema names, including
missing names, non-schema names, private imported schemas, self references,
forward references, and incompatible payload shapes.

`../../examples/specification/run/binary-schema-extension-dispatch-decode/`,
`../../examples/specification/run/binary-schema-extension-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
`../../examples/specification/run/binary-schema-imported-extension-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-extension-dispatch-unknown/`,
`../../examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
`../../examples/specification/run/binary-schema-imported-extension-dispatch-nested-unknown/`,
and
`../../examples/specification/run/binary-schema-extension-dispatch-length-human/`
pin the narrow extension-tolerant dispatch slice. The known case decodes the
selected exact-width, same-module nested, or public imported nested schema
payload into `SchemaDispatchPayload::Known`. The unknown cases preserve the
decoded tag and a bounded raw `ByteView` without reporting
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
`../../examples/specification/run/transport-socket-optional-accept-boundary/`
case covers `net::accept_or_end` returning `Some(stream)` and using that
stream with the existing read behavior, while
`../../examples/specification/run/transport-socket-optional-accept-clean-end/`
covers clean listener end returning `None`. The matching
`../../examples/specification/check/transport-socket-effects/` case pins
missing-effect diagnostics for the socket calls, including the optional
clean-end listener accept and stream read,
`../../examples/specification/check/transport-socket-optional-accept-effects/`
pins the optional accept directly, and
`../../examples/specification/check/transport-socket-clean-end-effects/` pins
the optional clean-end read directly. The
`../../examples/specification/run/transport-socket-read-failure-human/`,
`../../examples/specification/run/transport-socket-read-failure-json/`,
`../../examples/specification/run/transport-socket-read-or-end-failure-json/`,
`../../examples/specification/run/transport-socket-optional-accept-failure-json/`,
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
and
`../../examples/specification/check/stream-adapter-cancellable-routing-effects/`
cases compose those wait outcomes with channel-routed `StreamInput` values and
ordinary response action values. The main routing case pins completed wait,
deadline-expired, and cancelled paths in one fixture output; the deadline case
also pins the global host-forced deadline expiry fixture. Together they show
that these outcomes become adapter decisions rather than runtime failures, and
that the adapter declares both `time` and `concurrency` while the handler
boundary stays free of transport effects.
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
`../../examples/specification/run/socket-stream-adapter-routing/` covers the
narrow adapter-owned socket-to-handler routing and stream-task handler slice.
It reads multiple fixture-backed `ByteChunk` values from one `NetStream`,
sends ordinary stream events through a standard channel under `concurrency`,
calls the plain handler with explicit state across those events, joins a
spawned stream-handler task over the same event/action boundary, and
translates ordered `SendBytes` response actions into `net::write_chunk` calls.
The handler has no socket handle and performs no `net` calls. The matching
`../../examples/specification/check/socket-stream-adapter-routing-effects/`
case pins that adapter-owned routing must declare the existing `net` and
`concurrency` effects for socket, channel, and task calls instead of adding a
new routing effect, while the plain handler boundary stays free of `net`.

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
function. The adapter creates and owns the `NetListener`, accepts an optional
`NetStream` with `net::accept_or_end`, reads accepted stream chunks until
clean end with `net::read_chunk_or_end`, routes ordinary stream input values
through a channel, calls a pure handler without exposing socket handles, and
projects `SendBytes` response actions back to ordered `net::write_chunk`
calls. The adapter declares the existing coarse `net` and `concurrency`
effects; the handler remains free of `net` calls.

The executable specification cases
`../../examples/specification/run/channel-first-stream-routing/` and
`../../examples/specification/run/channel-first-stream-routing-three-route/`
and
`../../examples/specification/run/channel-first-stream-routing-four-route/`
and
`../../examples/specification/run/channel-first-stream-routing-five-route/`
cover channel-first selection between ordinary `StreamInput` routes before
handler invocation. They use existing typed channels and
`channel::select_priority` or `channel::select_many_priority`, then call a
plain stream handler with explicit per-stream state. The matching
`../../examples/specification/check/channel-first-stream-routing-effects/`
and
`../../examples/specification/check/channel-first-stream-routing-three-route-effects/`
and
`../../examples/specification/check/channel-first-stream-routing-four-route-effects/`
and
`../../examples/specification/check/channel-first-stream-routing-five-route-effects/`
cases pin the effect boundary: the routing adapter requires `concurrency`,
socket wrappers around it require both `net` and `concurrency`, and the
handler boundary remains free of transport effects.

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
reuses the binary frame-header primitive for each available header after the
preface is consumed.

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
different frame kind and a different stream id, closed input while a header
block remains pending, an
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
outstanding local SETTINGS as `http2.protocol.unexpected_settings_ack`,
wrong-length SETTINGS ACK as a typed payload-length failure, SETTINGS ACK on a
nonzero stream as a stream id domain failure, PING frames with and without ACK,
wrong-length PING failures with inspected-payload byte previews, a PRIORITY
frame that exposes dependency stream id, exclusive flag, and weight facts in
the frame value and tracked open-stream state, replacement of those tracked
facts by a later PRIORITY frame for the same stream, PRIORITY stream-state
failures for idle, closed-by-peer, reset, and mismatched streams, PRIORITY
stream id zero, wrong-length, and self-dependency failures, a GOAWAY
frame that moves the connection into
graceful shutdown with last-stream-id and error-code facts, wrong-length
GOAWAY failures, and `RST_STREAM` receive behavior for open, zero-id,
wrong-length, idle-stream, and reset-then-stream-frame cases.
Pending continuation state records the owning stream, starting frame kind,
starting byte offset, and accumulated opaque header-block bytes, and the
closed-input continuation failure projects that context into the stable output.
Receive-limit state records the active maximum frame size with
protocol-default, local-configuration, or local-SETTINGS provenance.
Receive flow-control state records connection receive-window credit and the
currently open stream receive-window credit. DATA on the open stream consumes
both windows by payload length. PADDED DATA consumes receive-window credit for
the full DATA payload, including the pad-length byte and padding bytes, while
the exposed DATA content contains only application data bytes. A pad length
that exceeds the remaining DATA payload is reported as
`http2.protocol.invalid_data_padding`. Accepted DATA with `END_STREAM`, and
accepted HEADERS sequences with `END_STREAM` after header-block completion,
move the tracked stream to closed-by-peer state. Later DATA or stream-level
`WINDOW_UPDATE` for that stream uses the same stream-state failure shape as
other non-open stream states. `WINDOW_UPDATE` on the connection stream
increases connection receive-window credit, and `WINDOW_UPDATE` on the open
stream increases that stream's receive-window credit. A received
`SETTINGS_INITIAL_WINDOW_SIZE` item applies the delta from the previous active
peer setting to the currently open stream receive-window credit; the adjusted
credit can become negative, and DATA remains blocked until `WINDOW_UPDATE`
restores enough stream credit. Wrong-length `WINDOW_UPDATE` payloads remain
typed payload-length failures, idle-stream `WINDOW_UPDATE` remains the existing
stream-state frame-kind failure, and zero or overflowing increments remain
typed peer-limit failures without changing window state. DATA payloads larger
than the available stream or connection receive-window credit also remain
typed peer-limit failures. `RST_STREAM` on the open stream decodes its
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
peer-limit failures at the offending SETTINGS item byte offset. SETTINGS ACK
frames do not update peer-advertised state or receive-window credit. A valid
SETTINGS ACK clears outstanding local SETTINGS state; an ACK with no
outstanding local SETTINGS is a typed protocol failure. A
final CONTINUATION with END_HEADERS clears continuation state and exposes the
completed accumulated header-block bytes in observable example output.
The same HPACK fixture boundary accepts the static indexed `0x82`
`:method: GET`, `0x83` `:method: POST`, `0x84` `:path: /`, `0x85`
`:path: /index.html`, `0x86` `:scheme: http`, `0x87` `:scheme: https`,
`0x88` `:status: 200`, `0x89` `:status: 204`, `0x8a` `:status: 206`,
`0x8b` `:status: 304`, `0x8c` `:status: 400`, `0x8d` `:status: 404`, and
`0x8e` `:status: 500`, plus `0x8f` `accept-charset:`,
`0x90` `accept-encoding: gzip, deflate`, and `0x91` `accept-language:`
header-block bytes in completed HEADERS frames, exposes
the decoded header name and value through ordinary header-list accessors,
advances the immutable fixture state, and keeps unsupported HPACK input on
`hpack.fixture.unsupported_header_block`.
The outbound DATA send-intent slice keeps outbound connection and stream
credit separate from inbound receive windows. It accepts a DATA intent within
the peer-advertised maximum frame size and available outbound connection and
stream windows, emits one immutable DATA frame-header-plus-payload chunk, then
consumes both outbound credits by payload length. It rejects DATA intents that
exceed the received `SETTINGS_MAX_FRAME_SIZE`, the available outbound
connection credit, or the peer-advertised stream credit derived from received
`SETTINGS_INITIAL_WINDOW_SIZE` before output bytes are emitted. Accepted DATA
with `END_STREAM` records local closed-stream state; later outbound DATA,
outbound HEADERS, and stream-level outbound `WINDOW_UPDATE` for that stream
use the same closed stream-state rejection boundary. Generated DATA
frame-header representation failures remain codec encode errors.
The local SETTINGS send-intent slice emits exactly one SETTINGS item per
intent for `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
`SETTINGS_MAX_FRAME_SIZE`, or `SETTINGS_MAX_HEADER_LIST_SIZE`. Each accepted
intent emits a
frame-header-plus-item chunk with length `6`, kind `4`, flags `0`, stream id
`0`, the selected setting identifier, and the selected four-byte unsigned
value, then records one outstanding local SETTINGS batch. Local
`SETTINGS_ENABLE_PUSH` values outside `0..1` are rejected before bytes are
emitted with the SETTINGS range failure shape. A valid SETTINGS ACK clears
that outstanding state, and an ACK with no outstanding local SETTINGS stays on
the typed unexpected-ACK failure path.
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
header-block chunk for a nonzero currently open stream, emits a frame-header
plus header-block output chunk with kind `1` and `END_HEADERS`, optionally
sets `END_STREAM`, and records local closed-stream state after an accepted
`END_STREAM` intent. It rejects stream id `0`, missing streams, closed
streams, already reset streams, mismatched open streams, payloads larger than
the peer-advertised maximum frame size, and generated frame-header
representation failures before accepted bytes are produced.
The outbound GOAWAY send-intent slice accepts a last stream id and error code,
emits a frame-header plus GOAWAY payload output chunk with length `8`, kind
`7`, flags `0`, and stream id `0`, then records local graceful-shutdown state
so a later peer-created HEADERS stream greater than the sent last stream id
follows the existing post-GOAWAY stream rejection boundary. It preserves
generated encode-helper representation failures for the last stream id or
error-code payload before accepted bytes are produced.
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
`END_STREAM`, an accepted GOAWAY frame plus last-stream-id and error-code
payload, and the maximum valid `UInt31be` stream id. The source
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
`../../examples/specification/run/http2-protocol-core-preface-partial-human/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-human/`,
`../../examples/specification/run/http2-protocol-core-continuation-json/`,
`../../examples/specification/run/http2-protocol-core-frame-size-human/`,
`../../examples/specification/run/http2-protocol-core-settings-value-human/`,
`../../examples/specification/run/http2-protocol-core-flow-control-human/`,
`../../examples/specification/run/http2-protocol-core-data-padding-human/`,
`../../examples/specification/run/http2-protocol-core-concurrent-streams-human/`,
`../../examples/specification/run/http2-protocol-core-invalid-stream-id-human/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-push-promise-human/`,
`../../examples/specification/run/http2-protocol-core-settings-ack-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-settings-unexpected-ack-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-ping-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-goaway-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-frame-size-json/`,
`../../examples/specification/run/http2-protocol-core-preface-partial-json/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-json/`,
`../../examples/specification/run/http2-protocol-core-settings-value-json/`,
`../../examples/specification/run/http2-protocol-core-flow-control-json/`,
`../../examples/specification/run/http2-protocol-core-data-padding-json/`,
`../../examples/specification/run/http2-protocol-core-concurrent-streams-json/`,
`../../examples/specification/run/http2-protocol-core-invalid-stream-id-json/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-push-promise-json/`,
`../../examples/specification/run/http2-protocol-core-settings-ack-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-settings-unexpected-ack-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-ping-length-json/case.toml`,
`../../examples/specification/run/http2-protocol-core-priority-dependency-json/case.toml`,
and `../../examples/specification/run/http2-protocol-core-goaway-length-json/case.toml`
pin the command-facing projection path for those typed failures. The human
cases check focused primary messages and related context, while the JSON cases
check `protocol_diagnostic` details for byte offset, frame kind, stream id,
active continuation, connection state, or stream state, observed and allowed
frame sizes, setting identity, observed setting value, accepted setting range,
stream reference, receive-limit provenance, peer-limit provenance, observed and
expected payload length including SETTINGS ACK length zero and `RST_STREAM`
length four, unexpected SETTINGS ACK state, flow-control window credit,
expected and actual
preface byte values, matched preface prefix count, expected preface byte count,
structured bounded preface and invalid-payload byte preview fields,
concurrent-stream attempted and allowed counts, required stream id domain,
endpoint role, PRIORITY dependency stream id, and rule provenance. The
preface human cases also check nearby-byte notes rendered as
bounded lowercase hex pairs with total byte count and truncation state. The
concurrent-stream command fixtures cover the focused peer-created stream limit
projection, and the flow-control command fixtures cover stream
receive-window provenance while the ordinary protocol-core case also covers
connection receive-window provenance and the `WINDOW_UPDATE` receive-credit
slice.
The frame-size command fixtures cover local-configuration provenance while the
ordinary protocol-core case keeps the protocol-default, local-configuration,
local-SETTINGS, peer-advertised SETTINGS, rejected peer-advertised SETTINGS,
and peer-advertised initial-window receive-window distinctions visible in
executable output.
