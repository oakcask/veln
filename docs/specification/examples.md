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
`../../examples/specification/run/derived-codec-nested-dispatch-decode-boundary/`
covers the same derived codec call boundary when the generated decode-step
helper decodes a same-module nested dispatch payload schema.
The executable specification case
`../../examples/specification/check/derived-codec-mapping-boundary-diagnostics/`
covers mapped derived encode clauses whose generated helper boundary cannot
accept the schema mapping target value type.

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
The executable specification case
`../../examples/specification/run/derived-codec-nested-dispatch-encode-boundary/`
covers the same derived codec call boundary when the generated encode helper
writes a same-module nested dispatch payload schema and projects dispatch
selection failures as `Invalid(EncodeError)`.
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
accepted `UInt24le` and `UInt32le` `format binary` field use.
`../../examples/specification/run/binary-schema-little-endian-widths-decode/`,
`../../examples/specification/run/binary-schema-little-endian-widths-encode/`,
and
`../../examples/specification/run/binary-schema-little-endian-widths-encode-out-of-range/`
cover the `UInt24le` and `UInt32le` slice. The runtime cases prove
little-endian decode and encode byte order, preserve structural mapping during
decode, and pin generated encode helper range failures with maximum values
derived from each primitive width.

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
`../../examples/specification/run/binary-schema-packed-reserved-decode/` and
`../../examples/specification/run/binary-schema-packed-reserved-json/` pin the
one-byte packed reserved-bit decode slice. The valid case decodes high
`ReservedBits(width, value)` prefixes for widths one through seven plus the
visible `UIntN` field that completes each byte, omits the reserved field from
the decoded record, and then reads the following field at the next byte. The
failing case asserts `schema.reserved_bits_mismatch` for the packed reserved
field. The checked diagnostics case
`../../examples/specification/check/schema-packed-reserved-mapping-diagnostics/`
asserts that the packed reserved field is not available as a structural
mapping source field.

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
`../../examples/specification/run/binary-schema-packed-reserved-encode/` pins
one-byte packed reserved-bit encode for widths one through seven: the helper
writes high reserved bits from the declaration and low visible bits from the
source value record in one byte.

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
calls under the coarse `net` effect. The matching
`../../examples/specification/check/transport-socket-effects/` case pins
missing-effect diagnostics for the socket calls. The
`../../examples/specification/run/transport-socket-read-failure-human/`,
`../../examples/specification/run/transport-socket-read-failure-json/`,
`../../examples/specification/run/transport-socket-write-failure-human/`, and
`../../examples/specification/run/transport-socket-write-failure-json/` cases
show read and write failures as runtime transport failures, not schema, codec,
or peer protocol diagnostics.

The executable specification cases
`../../examples/specification/run/transport-boundary/`,
`../../examples/specification/run/transport-deadline/`,
`../../examples/specification/run/transport-cancellable-wait/`, and
`../../examples/specification/check/transport-cancellable-wait-effects/`
cover descriptor-backed time waits, relative deadlines, and source-visible
`CancelToken` values under the existing `time` effect. The
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
narrow adapter-owned socket-to-handler routing slice. It reads one
fixture-backed `ByteChunk` from a `NetStream`, sends an ordinary stream event
through a standard channel under `concurrency`, calls the plain handler, and
translates ordered `SendBytes` response actions into `net::write_chunk` calls.
The handler has no socket handle and performs no `net` calls. The matching
`../../examples/specification/check/socket-stream-adapter-routing-effects/`
case pins that adapter-owned routing still uses the existing `concurrency`
effect for channel calls instead of a new routing effect.

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
frame that exposes dependency stream id, exclusive flag, and weight facts,
PRIORITY stream id zero, wrong-length, and self-dependency failures, a GOAWAY
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
both windows by payload length. Accepted DATA with `END_STREAM`, and accepted
HEADERS sequences with `END_STREAM` after header-block completion, move the
tracked stream to closed-by-peer state. Later DATA or stream-level
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
The same HPACK fixture boundary accepts the static indexed `:method: GET`
header-block byte in a completed HEADERS frame, exposes the decoded header
name and value through ordinary header-list accessors, advances the immutable
fixture state, and keeps unsupported HPACK input on
`hpack.fixture.unsupported_header_block`.
The outbound DATA send-intent slice keeps outbound connection and stream
credit separate from inbound receive windows. It accepts a DATA intent within
the peer-advertised maximum frame size and available outbound connection and
stream windows, then consumes both outbound credits by payload length. It
rejects DATA intents that exceed the received `SETTINGS_MAX_FRAME_SIZE`, the
available outbound connection credit, or the peer-advertised stream credit
derived from received `SETTINGS_INITIAL_WINDOW_SIZE`.
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
on the connection stream, a DATA header on a nonzero stream, a local SETTINGS
frame-header-plus-item chunk, an accepted `RST_STREAM` frame plus error-code
payload, an accepted GOAWAY frame plus last-stream-id and error-code payload,
and the maximum valid `UInt31be` stream id. The source output also matches generated helper
`codec.encode_value_unrepresentable` failure for an out-of-range stream id,
keeping field path and reason text visible without converting it into a
protocol diagnostic.

`../../examples/specification/run/http2-protocol-core-closed-human/`,
`../../examples/specification/run/http2-protocol-core-preface-partial-human/`,
`../../examples/specification/run/http2-protocol-core-preface-invalid-human/`,
`../../examples/specification/run/http2-protocol-core-continuation-json/`,
`../../examples/specification/run/http2-protocol-core-frame-size-human/`,
`../../examples/specification/run/http2-protocol-core-settings-value-human/`,
`../../examples/specification/run/http2-protocol-core-flow-control-human/`,
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
