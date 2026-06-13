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
The executable specification case
`../../examples/specification/run/binary-schema-decode-step/` covers the
generated schema-derived decode-step helper: complete buffered input returns
`Decoded` with the exact consumed count, and short open input returns
`NeedMore(NeedBytes(...))` without consuming bytes.
The executable specification case
`../../examples/specification/run/codec-decode-boundary/` covers a
hand-written codec decode boundary: a codec item call passes `ByteView` and
`ByteOffset` to the referenced decoder and observes its returned `Decoded`,
`NeedMore`, and `Invalid` `DecodeStep<T>` values unchanged while the schema
mapping pins the accepted value type.
The executable specification case
`../../examples/specification/run/derived-codec-decode-boundary/` covers a
derived codec decode boundary for the same eligible generated binary schema
decode-step slice: a codec item call observes the generated helper's
`Decoded`, `NeedMore`, and `Invalid` `DecodeStep<T>` values through the codec
item name while preserving mapped record fields and no-consumption outcomes.

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
its returned `Encoded` and `Invalid(EncodeError)` `EncodeStep<TState>` values
unchanged.
The executable specification case
`../../examples/specification/run/derived-codec-encode-boundary/` covers a
derived codec encode boundary for the eligible generated binary schema encode
helper slice: a codec item call observes successful helper output as
`Encoded(List<ByteChunk>)` with one chunk and out-of-range generated helper
failures as `Invalid(EncodeError)`.

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

`../../examples/specification/run/binary-schema-primitive-encode/` and
`../../examples/specification/run/binary-schema-primitive-encode-out-of-range/`
pin the generated exact-width primitive encode helper slice. The passing case
encodes `UInt16be` followed by `UInt32be` into one immutable `ByteChunk` and
checks complete lowercase hex output. The failing case matches the returned
`EncodeError` and asserts `codec.out_of_range`, the schema field path, and the
`UInt31be` maximum.

`../../examples/specification/run/binary-schema-reserved-bit-encode/` pins the
reserved-bit encode slice for `ReservedBits(1, 0)` followed by `UInt31be`.
The case checks complete lowercase hex output for an HTTP/2-style stream
identifier field and the `UInt31be` maximum boundary. The adjacent checker
case
`../../examples/specification/check/schema-reserved-bit-encode-diagnostics/`
asserts `schema.reserved_bits_encode` for a reserved-bit shape outside the
implemented encode layout.

`../../examples/specification/run/binary-schema-closed-dispatch-decode/`,
`../../examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
and
`../../examples/specification/run/binary-schema-closed-dispatch-unknown-human/`
pin the narrow closed dispatch slice. The passing case decodes a known tag and
selected primitive payload as ordinary `Int` fields; the nested passing case
decodes the selected same-module payload schema as a record-shaped field. The
failing cases assert
`schema.dispatch_unknown_tag`, the dispatch byte offset, structured field
path, decoded tag field and value, expected tag values, structured byte
preview fields, and focused human related notes.
`../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`
pins the static boundary for nested dispatch payload schema names, including
missing names, non-schema names, imported schemas, self references, forward
references, and incompatible payload shapes.

`../../examples/specification/run/binary-schema-extension-dispatch-decode/`,
`../../examples/specification/run/binary-schema-extension-dispatch-nested-decode/`,
`../../examples/specification/run/binary-schema-extension-dispatch-unknown/`,
`../../examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
`../../examples/specification/run/binary-schema-dispatch-nested-failure-json/`,
and
`../../examples/specification/run/binary-schema-extension-dispatch-length-human/`
pin the narrow extension-tolerant dispatch slice. The known case decodes the
selected exact-width or same-module nested schema payload into
`SchemaDispatchPayload::Known`. The unknown cases preserve the decoded tag and
a bounded raw `ByteView` without reporting `schema.dispatch_unknown_tag`. The
nested failure case pins the nested schema field path and absolute byte offset.
The malformed structural case still reports `schema.length_out_of_bounds` when
the decoded length cannot be sliced from closed input.

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
incomplete frame input that waits for more bytes, closed input with pending
frame bytes, continuation state after HEADERS, continuation state after a
non-final CONTINUATION, completion after a final CONTINUATION, one
continuation ordering failure, and an incoming frame whose payload length
exceeds the active receive maximum frame size, plus a DATA frame kind rejected
for connection-control state and idle-stream state. It also pins PING frames
with and without ACK, wrong-length and stream-targeted PING failures, a GOAWAY
frame that moves the connection into graceful shutdown with last-stream-id and
error-code facts, and wrong-length and stream-targeted GOAWAY failures.
Pending continuation state records the owning stream, starting frame kind,
starting byte offset, and accumulated opaque header-block byte count.
Receive-limit state records the active maximum frame size with
protocol-default, local-configuration, or local-SETTINGS provenance.
Receive flow-control state records connection receive-window credit and the
currently open stream receive-window credit. DATA on the open stream consumes
both windows by payload length. `WINDOW_UPDATE` on the connection stream
increases connection receive-window credit, and `WINDOW_UPDATE` on the open
stream increases that stream's receive-window credit. Wrong-length
`WINDOW_UPDATE` payloads remain typed payload-length failures, idle-stream
`WINDOW_UPDATE` remains the existing stream-state frame-kind failure, and zero
or overflowing increments remain typed peer-limit failures without changing
window state. DATA payloads larger than the available stream or connection
receive-window credit also remain typed peer-limit failures.
Peer-received `SETTINGS_MAX_FRAME_SIZE` is stored as peer-advertised state for
outbound decisions and does not replace the inbound receive maximum used by
later frame-size checks. Received `SETTINGS_MAX_FRAME_SIZE` values are
range-checked before updating peer-advertised state; out-of-range values stay
as typed peer-limit failures at the offending SETTINGS item byte offset. A
final CONTINUATION with END_HEADERS clears continuation state and exposes the
completed accumulated byte count in the observable example output. Protocol
failures stay as ordinary ADT values and are projected by source code into
stable diagnostic ids and related context fields for byte offset, observed and
allowed lengths, actual and expected frame kind, stream reference, active
continuation, connection state, or stream state, setting identity, accepted
SETTINGS range, receive-limit provenance, peer-limit provenance, payload length
expectations, matched preface prefix count, expected and actual preface byte,
and rule provenance.
The same case also pins outbound frame header encoding from an ordinary
record-shaped frame description through the generated binary schema encode
helper. The checked `[[output_chunk_list]]` fixtures cover a SETTINGS header
on the connection stream, a DATA header on a nonzero stream, and the maximum
valid `UInt31be` stream id. The source output also matches a generated helper
`codec.out_of_range` failure for an out-of-range stream id, keeping field path
and reason text visible without converting it into a protocol diagnostic.

`../../examples/specification/run/http2-protocol-core-closed-human/`,
`../../examples/specification/run/http2-protocol-core-preface-partial-human/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-human/`,
`../../examples/specification/run/http2-protocol-core-continuation-json/`,
`../../examples/specification/run/http2-protocol-core-frame-size-human/`,
`../../examples/specification/run/http2-protocol-core-settings-value-human/`,
`../../examples/specification/run/http2-protocol-core-flow-control-human/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-human/`,
`../../examples/specification/run/http2-protocol-core-ping-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-goaway-length-human/case.toml`,
`../../examples/specification/run/http2-protocol-core-frame-size-json/`,
`../../examples/specification/run/http2-protocol-core-preface-partial-json/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-json/`,
`../../examples/specification/run/http2-protocol-core-settings-value-json/`,
`../../examples/specification/run/http2-protocol-core-flow-control-json/`,
`../../examples/specification/run/http2-protocol-core-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-stream-invalid-frame-kind-json/`,
`../../examples/specification/run/http2-protocol-core-ping-length-json/case.toml`,
and `../../examples/specification/run/http2-protocol-core-goaway-length-json/case.toml`
pin the command-facing projection path for those typed failures. The human
cases check focused primary messages and related context, while the JSON cases
check `protocol_diagnostic` details for byte offset, frame kind, stream id,
active continuation, connection state, or stream state, observed and allowed
frame sizes, setting identity, observed setting value, accepted setting range,
stream reference, receive-limit provenance, peer-limit provenance, observed and
expected payload length, flow-control window credit, expected and actual
preface byte values, matched preface prefix count, expected preface byte count,
and rule provenance. The flow-control command fixtures cover stream
receive-window provenance while the ordinary protocol-core case also covers
connection receive-window provenance and the `WINDOW_UPDATE` receive-credit
slice.
The frame-size command fixtures cover local-configuration provenance while the
ordinary protocol-core case keeps the protocol-default, local-configuration,
local-SETTINGS, peer-advertised SETTINGS, and rejected peer-advertised SETTINGS
distinctions visible in executable output.
