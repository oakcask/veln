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
`ByteView` slices, checked unsigned big-endian reads, checked unsigned
big-endian writes, truncation failures, range failures, and conversion
overflow failures without relying on HTTP/2 or codec declarations. It also
passes a `ByteView` through a channel and reads the received view to cover the
ordinary immutable freeze boundary.

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

## Codec Encode Step Vocabulary

The executable specification case
`../../examples/specification/run/codec-encode-step-vocabulary/` covers the
source-visible incremental encode transition vocabulary. Ordinary source
functions construct `EncodeStep<TState>` values for complete `Encoded`
`List<ByteChunk>` output, `Partial` committed chunks with produced
`ByteCount` and resumable state, and an `Invalid` outcome carrying a
structured `EncodeError` with id, field path, and representation-failure
reason.

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

`../../examples/specification/run/binary-fixed-field-mismatch-json/` and
`../../examples/specification/run/binary-fixed-field-mismatch-human/` pin the
first schema-owned fixed-field mismatch diagnostic slice. The JSON case
asserts `schema.fixed_field_mismatch`, decoded byte offset, structured field
path, expected and actual byte values, and structured byte preview fields. The
human case asserts that the primary message stays focused on the fixed-field
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

`../../examples/specification/run/binary-schema-validation-arithmetic-decode/`
and `../../examples/specification/run/binary-schema-validation-arithmetic-json/`
pin generated `byte_decode_<schema>` helpers for another schema declaration.
The passing case decodes an exact-width arithmetic predicate. The failing case
asserts the same `schema.validation_failed` shape with decoded values keyed by
schema field name.

`../../examples/specification/run/binary-schema-mapped-record-decode/` pins
the generated schema mapping slice. The helper decodes exact-width schema
fields, checks the field-local predicate, and returns the mapped ordinary
record field names rather than the schema-local field names.

## HTTP/2 Protocol Core Example

The executable specification case
`../../examples/specification/run/http2-protocol-core/` shows the implemented
ordinary-source HTTP/2 sans-I/O decode-state slice. The example models input
chunks and end-of-stream as explicit ADT events, stores parser state as the
undecoded `ByteChunk` suffix plus the next absolute byte offset, and reuses
the binary frame-header primitive for each available header.

The case pins valid frame arrival, incomplete input that waits for more bytes,
closed input with pending bytes, continuation state after HEADERS, continuation
state after a non-final CONTINUATION, completion after a final CONTINUATION,
one continuation ordering failure, and an incoming frame whose payload length
exceeds the active receive maximum frame size, plus a DATA frame kind rejected
for connection-control state and idle-stream state. Pending continuation state
records the owning stream, starting frame kind, starting byte offset, and
accumulated opaque header-block byte count. A final CONTINUATION with
END_HEADERS clears that state and exposes the completed accumulated byte count
in the observable example output. Protocol failures stay as ordinary ADT
values and are projected by source code into stable diagnostic ids and related
context fields for byte offset, observed and allowed lengths, actual and
expected frame kind, stream reference, active continuation, connection state,
or stream state, receive-limit provenance, and rule provenance.

`../../examples/specification/run/http2-protocol-core-closed-human/`,
`../../examples/specification/run/http2-protocol-core-continuation-json/`,
`../../examples/specification/run/http2-protocol-core-frame-size-human/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-frame-size-json/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-json/`, and
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-json/`
pin the command-facing projection path for those typed failures. The human
cases check focused primary messages and related context, while the JSON cases
check `protocol_diagnostic` details for byte offset, frame kind, stream id,
active continuation, connection state, or stream state, observed and allowed
frame sizes, stream reference, receive-limit provenance, and rule provenance.
