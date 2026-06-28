# Binary Schema Primitives And Dispatch

Status: proposed

This proposal defines the remaining binary-schema field vocabulary needed for
frame headers and frame-specific payload dispatch. It depends on a schema
declaration surface and a byte standard-library vocabulary.

The source-surface `ReservedBits(width, value)` declaration syntax is
implemented under `../specification/source-surface.md`.
The declaration-time exact-width primitive names `UInt1` through `UInt8`,
`UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`, `UInt31le`,
`UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`,
`UInt56be`, `UInt56le`, `UInt64be`, and `UInt64le`
are also implemented there for `format binary` schema field type positions
only. The generated
`Http2FrameHeaderWire` helper slice is implemented under
`../specification/execution.md`: it consumes `UInt24be`, `UInt8`, `UInt8`,
`ReservedBits(1, 0)`, and `UInt31be` from a `ByteView`, returns ordinary
`Int` fields for the visible values, and reports structured schema failures
for truncated fields and reserved-bit mismatches. The HTTP/2 protocol-core
frame-header decode path and focused protocol diagnostic projection examples
declare and call that generated helper instead of the former source-visible
`byte_decode_http2_frame_header` prelude helper. The narrow runtime method
remains only as internal compatibility for the source-visible
`byte_decode_http2_frame` payload-boundary helper until that payload slice is
also generalized. Generated schema helpers
also consume byte-aligned
`ReservedBits(width, value)` fields up to four bytes wide as
representation-only fields, omit those fields from decoded records and
mapping source values, encode them from the declared fixed value, and report
the same reserved-bit mismatch and truncation diagnostic shapes. Generated
schema helpers also consume and encode packed `ReservedBits(width, value)`
prefixes where widths one through seven are followed by the visible `UIntN`
primitive that completes the byte, widths nine through fifteen complete the
same two-byte big-endian storage unit, and widths seventeen through
twenty-three complete the same three-byte big-endian storage unit, and widths
twenty-five through thirty-one complete the same four-byte big-endian storage
unit. The helpers
validate the high reserved bits, decode or encode the low visible bits from
the ordinary record field, omit the reserved field from decoded records and
mapping source values, and report the same reserved-bit mismatch, truncation,
and `codec.encode_value_unrepresentable` diagnostic shapes. Helper history
for the `ReservedBits(15, value)` plus `UInt1` two-field boundary is recorded
in
[Binary Schema Reserved Fifteen-Bit Prefix](../reference/implemented-proposals/binary-schema-reserved-fifteen-bit-prefix.md).
Generated schema helpers also consume and encode the suffix form where a
visible `UIntN` field is followed immediately by
`ReservedBits(width, value)` and the two widths complete one byte or the same
two-byte, three-byte, or four-byte big-endian storage unit, plus the five-byte
case where the fields complete forty bits and the six-byte case where the
fields complete forty-eight bits, the seven-byte case where the fields
complete fifty-six bits, and the eight-byte case where the fields complete
sixty-four bits.
The helpers decode or encode the visible field from the high bits, validate or
emit the declared low reserved bits, omit the reserved field from decoded
records and mapping source values, and report the same reserved-bit mismatch,
truncation, and `codec.encode_value_unrepresentable` diagnostic shapes.
The completed one-byte reserved suffix slice is archived under
`../reference/implemented-proposals/binary-schema-one-byte-reserved-suffix.md`.
The completed six-byte reserved suffix slice is archived under
`../reference/implemented-proposals/binary-schema-six-byte-reserved-suffix.md`.
The completed seven-byte and eight-byte reserved suffix slice is archived
under
[Binary Schema Wide Reserved Suffix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-suffix-groups.md).
Generated schema helpers also consume and encode one-byte, two-byte,
three-byte, four-byte, five-byte, six-byte, seven-byte, and eight-byte
big-endian reserved prefix
groups where
`ReservedBits(width, value)` is followed by two visible sub-byte or
byte-width `UIntN` fields and all three widths complete the storage unit. The
two-byte form includes reserved prefix widths one through fourteen when the
visible fields complete the remaining bits, the three-byte form includes
reserved prefix widths seventeen through twenty-three when the visible fields
complete the remaining bits, and the four-byte form includes reserved prefix
widths twenty-five through thirty-one when the visible fields complete the
remaining bits. The five-byte form includes reserved prefix width
thirty-three when the visible fields complete the remaining bits, and the
six-byte form includes reserved prefix width forty-one when the visible
fields complete the remaining bits. The seven-byte form includes reserved
prefix width forty-nine when the visible fields complete the remaining bits,
and the eight-byte form includes reserved prefix width fifty-seven when the
visible fields complete the remaining bits.
The helpers
validate or emit the high reserved bits, decode or
encode the two visible
fields from high to low, omit the reserved field from decoded records and
mapping source values, and report the same reserved-bit mismatch, truncation,
and `codec.encode_value_unrepresentable` diagnostic shapes.
The completed seven-byte and eight-byte reserved prefix group slice is
archived under
`../reference/implemented-proposals/binary-schema-wide-reserved-prefix-groups.md`.
Generated schema helpers also consume and encode the narrow two-byte
byte-interleaved middle layout where one visible sub-byte `UIntN` field is
followed by one sub-byte `ReservedBits(width, value)` field, one visible
`UInt8` field, and one final visible sub-byte `UIntN` field whose widths
complete the same two-byte big-endian storage unit. The helpers
preserve all visible fields, omit the reserved field from decoded records and
mapping source values, validate or emit the declared reserved bits at the
middle field position, include the layout in derived codec eligibility, and
report the same reserved-bit mismatch, truncation, and
`codec.encode_value_unrepresentable` diagnostic shapes.
Generated schema helpers also consume and encode the narrow
`ReservedBits(9, 0)` plus `UInt8` byte-prefix layout as a two-byte
big-endian bitstream slice, omitting the reserved field from decoded records,
mapping source values, and encoder input records while preserving the visible
byte field and existing reserved-bit mismatch, truncation, and
`codec.encode_value_unrepresentable` diagnostic shapes.
Generated schema helpers also consume and encode consecutive
non-byte-aligned `UIntN` and `ReservedBits(width, value)` fields when the
group contains at least one visible field and at least one reserved field and
the declared widths complete one byte or one two-byte, three-byte, four-byte,
five-byte, six-byte, seven-byte, or eight-byte big-endian storage unit. The helpers
validate or emit each reserved field at its declared position, omit reserved
fields from decoded records and mapping source values, preserve visible fields
in declaration order, and report the same reserved-bit mismatch, truncation,
and `codec.encode_value_unrepresentable` diagnostic shapes.
Generated schema helpers also consume and encode the narrow two-byte suffix
group where two visible `UIntN` fields are followed by a non-byte-aligned
`ReservedBits(width, value)` suffix, the second visible field is `UInt8`, and
all three widths complete the same two-byte big-endian storage unit. The helpers
decode or encode the visible fields in declaration order, validate or emit
the low reserved bits, omit the reserved field from decoded records and
mapping source values, include the layout in derived codec eligibility, and
report the same reserved-bit mismatch and
`codec.encode_value_unrepresentable` diagnostic shapes.
The completed two-byte suffix reserved group slice is archived under
`../reference/implemented-proposals/binary-schema-suffix-reserved-groups.md`.
Generated schema
helpers also decode and encode standalone visible
`UInt1` through `UInt7` fields as one byte each, expose the declared low bits
as ordinary `Int` values, preserve structural mapping and generated
decode-step and derived codec eligibility, and report existing truncation and
`codec.encode_value_unrepresentable` range-failure shapes. Generated schema
helpers also decode and encode consecutive visible-only `UInt1` through
`UInt7` fields when at least two fields complete exactly one byte, packing the
fields in declaration order from high bits to low bits, preserving each
visible field as an ordinary `Int`, including generated decode-step and
derived codec eligibility, and reporting existing truncation and
`codec.encode_value_unrepresentable` shapes. Generated schema helpers also
decode and encode the narrow visible-only two-byte big-endian group where
consecutive `UInt1` through `UInt7` fields complete exactly one two-byte
storage unit, preserving the same high-to-low declaration-order packing,
ordinary `Int` fields, generated decode-step and derived codec eligibility,
truncation shape, and encode range-failure shape. The completed two-byte
visible-only group slice is archived under
`../reference/implemented-proposals/binary-schema-packed-visible-two-byte-groups.md`.
The
generated helper slice also treats visible exact-width fields with a
field-local equality predicate such as `field == literal` as schema-owned
fixed fields, leaves matching values visible in the decoded result, and
reports `schema.fixed_field_mismatch` with byte offset, field path, expected
value, actual value, and byte preview details when the input differs. The
width-sample primitive decode slice consumes
`UInt16be` and `UInt32be`, returns ordinary `Int` values, and reports the
same structured truncation shape. The narrow HTTP/2 frame helper also returns
a bounded payload `ByteView` selected by the decoded length and reports
`schema.length_out_of_bounds` when closed input cannot provide that payload
range. The generated helper slice also implements `UInt16le`, `UInt24le`,
`UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le`
as little-endian unsigned primitives for schema decode and encode helpers,
implements `UInt40be` as the matching big-endian five-byte primitive,
`UInt48be` as the matching big-endian six-byte primitive, `UInt56be` as the
matching big-endian seven-byte primitive, and `UInt64be` as the matching
big-endian eight-byte primitive, returns ordinary `Int` values when the
decoded value is representable as source-visible `Int`, preserves structural
decode mappings, and reports width-specific encode range failures. The narrow
closed dispatch slice implements
`Dispatch(tag_field, tag => Primitive, ...)` for generated binary schema
decode helpers, decodes known case payloads as `Int`, and reports
`schema.dispatch_unknown_tag` with structured tag and byte context for unknown
tags. The narrow extension-tolerant dispatch slice implements
`ExtensionDispatch(tag_field, length_field, tag => Primitive, ...)` for
generated binary schema decode helpers, decodes known case payloads as
`SchemaDispatchPayload::Known(Int)`, preserves unknown tags and bounded raw
payload bytes as `SchemaDispatchPayload::Unknown(tag, payload)`, and still
reports `schema.length_out_of_bounds` for malformed payload ranges. The
same-module nested payload slice also implements known
`Dispatch(..., tag => SchemaName, ...)` and
`ExtensionDispatch(..., tag => SchemaName, ...)` cases for generated binary
schema decode helpers, returns the nested schema's decoded record shape for
known cases, keeps extension-tolerant unknown tags opaque, and reports nested
payload failures with the nested schema field path and absolute byte offset.
Public imported nested binary schema payloads named through written `use`
paths are accepted by those same dispatch decode helper slices and decode to
the imported schema's record shape. Same-module and public imported recursive
closed-dispatch and extension-dispatch payload decode and encode slices are
implemented for the length-bounded forms when selected mappings cover every
known case, all mappings resolve to one record shape, and at least one case is
non-recursive; recursive decode failures keep the outer dispatch field segment
before nested schema field segments, recursive encode checks the encoded
payload byte count against the earlier length field, and extension-dispatch
unknown tags preserve bounded raw payload bytes. Imported private, missing,
wrong-kind, non-binary, forward, unbounded recursive, or otherwise ineligible
payload schemas, including schemas outside the generated helper slice, use the
existing `schema.dispatch_payload` diagnostic shape. Resolved binary payload
schemas outside that helper slice include structured expected decode and
encode helper fields plus related notes for the payload declaration; checked
coverage includes a nested payload whose `ByteView` length field is not an
earlier decoded `Int` field and a nested payload with an unsupported
representation-only `ReservedBits` layout, plus a mapped payload schema that
decodes but cannot project its mapping assignment back to schema-local fields
for generated encode.
Closed dispatch payload cases with mixed primitive and nested schema decoded
shapes are implemented for the selected mapping boundary when every selector
uses the dispatch tag field, every dispatch case has one distinct matching
selector literal, each selected branch type-checks `payload` against that
case's payload shape, and all selected mappings resolve to one target record
shape. Selectors outside the dispatch tag field and uncovered mixed cases
remain rejected through the existing dispatch payload or mapping selection
diagnostics.
The same-module and imported public nested payload encode slice implements known
`Dispatch(..., tag => SchemaName, ...)` and
`ExtensionDispatch(..., tag => SchemaName, ...)` cases for generated binary
schema encode helpers, uses the nested schema decoded record shape for closed
payload fields and `SchemaDispatchPayload<NestedRecord>` for
extension-tolerant payload fields, accepts public imported payload schemas
named through written `use` paths, preserves extension-tolerant unknown raw
payload bytes, and keeps nested schema encode failures on the nested schema
field path. Those nested dispatch payload decode and encode slices route
selected nested payload schemas through the same generated binary schema
helper path as ordinary schema fields; focused executable examples cover
fixed-field validation, byte-aligned reserved fields, little-endian primitive
payload fields, same-module representation-only reserved-bit payload
round trips, nested `ByteView(length_field)` payload fields whose length is an
earlier visible `Int` in the same nested schema, extension-tolerant known
payloads, recursive extension known payloads, unknown payload preservation,
and nested helper diagnostics. The completed nested dispatch
`ByteView(length_field)` payload helper slice is archived under
`../reference/implemented-proposals/binary-schema-dispatch-byteview-payload-helpers.md`.
The completed dispatch payload helper boundary diagnostics slice is archived
under
`../reference/implemented-proposals/binary-schema-dispatch-payload-helper-boundary-diagnostics.md`.
A
checked non-HTTP telemetry envelope combines the implemented helper vocabulary
in one generated decode-and-encode schema. The mapping slice also accepts an
ADT constructor target field whose constructor payload is selected from a
record-shaped schema mapping expression and keeps malformed selections rejected
through the existing `schema.mapping_expression_unsupported` diagnostic.
Broader unsupported field layouts and schema value mapping beyond the
implemented structural, constructor field-selection, and mapped-payload
eligibility diagnostic slices remain proposal work. The completed
reserved-byte-prefix encode slice for
`ReservedBits(2, 0)` and `ReservedBits(9, 0)` followed by `UInt8` is
archived under
`../reference/implemented-proposals/binary-schema-reserved-byte-prefix-encode.md`.
The narrow one-byte visible flag bitset slice is implemented as `Flag8` for
generated binary schema decode and encode helpers. `Flag8` consumes and emits
one byte through the existing `UInt8` representation path, decodes to the
source-visible `Flag8(bits: Int)` value instead of a raw `Int`, preserves
existing `UInt8` field behavior, shares exact-width truncation behavior, and
reports existing encode value-representation failures when `bits` cannot be
represented in one byte. The structural mapping slice also treats decoded
`Flag8` fields as schema-local `Flag8` values for direct target-field
assignment, same-module ADT constructor expressions, one pure same-module
converter call, and one imported public pure converter call through a written
`use` path or alias. Generated encode helpers keep schema-local `Flag8`
encode behavior and accept a projectable mapped-record encode boundary when
every visible encode field, such as `target_flags = flags`, can be projected
by a supported direct field, record-shaped, or field-selection mapping. They
also project the first narrow ADT
constructor inverse when a single target field wraps one schema-local `Flag8`
field or exact-width integer field, such as
`flags = Http2Flags(wire_flags)` or `kind = FrameKind(wire_kind)`, and
preserve the ordinary encode range-failure shape on the schema-local field
path.
Pure prelude helpers expose checked one-byte `Flag8` bit access through bit
indexes `0` through `7`, returning `Result` failures for out-of-range indexes
instead of masking or wrapping. Raw-bit helpers expose the wrapped integer
bits and construct `Flag8` values only for integers in the one-byte range,
returning `Result` failures before invalid values reach generated schema
encoders.
The narrow two-byte big-endian visible flag bitset slice is implemented as
`Flag16be` for generated binary schema decode and encode helpers. `Flag16be`
consumes and emits two bytes through the existing `UInt16be` representation
path, decodes to the source-visible `Flag16be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt16be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in two bytes.
Pure prelude helpers expose checked two-byte `Flag16be` bit access through bit
indexes `0` through `15`, returning `Result` failures for out-of-range indexes
instead of masking or wrapping. Raw-bit helpers expose the wrapped integer
bits and construct `Flag16be` values only for integers in the two-byte range,
returning `Result` failures before invalid values reach generated schema
encoders.
The narrow two-byte little-endian visible flag bitset slice is implemented as
`Flag16le` for generated binary schema decode and encode helpers. `Flag16le`
consumes and emits two bytes through the existing `UInt16le` representation
path, decodes to the source-visible `Flag16le(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt16le` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in two bytes.
Pure prelude helpers expose checked two-byte `Flag16le` bit access through bit
indexes `0` through `15`, returning `Result` failures for out-of-range indexes
instead of masking or wrapping. Raw-bit helpers expose the wrapped integer
bits and construct `Flag16le` values only for integers in the two-byte range,
returning `Result` failures before invalid values reach generated schema
encoders.
The narrow three-byte big-endian visible flag bitset slice is implemented as
`Flag24be` for generated binary schema decode and encode helpers. `Flag24be`
consumes and emits three bytes through the existing `UInt24be` representation
path, decodes to the source-visible `Flag24be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt24be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in three bytes.
Pure prelude helpers expose checked three-byte `Flag24be` bit access through
bit indexes `0` through `23`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag24be` values only for integers in the
three-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow three-byte little-endian visible flag bitset slice is implemented
as `Flag24le` for generated binary schema decode and encode helpers.
`Flag24le` consumes and emits three bytes through the existing `UInt24le`
representation path, decodes to the source-visible `Flag24le(bits: Int)`
value instead of a raw `Int`, preserves existing `UInt24le` field behavior,
shares exact-width truncation behavior, supports direct mapped-record decode
and encode, and reports existing encode value-representation failures when
`bits` cannot be represented in three bytes.
Pure prelude helpers expose checked three-byte `Flag24le` bit access through
bit indexes `0` through `23`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag24le` values only for integers in the
three-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow four-byte big-endian visible flag bitset slice is implemented as
`Flag32be` for generated binary schema decode and encode helpers. `Flag32be`
consumes and emits four bytes through the existing `UInt32be` representation
path, decodes to the source-visible `Flag32be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt32be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in four bytes.
Pure prelude helpers expose checked four-byte `Flag32be` bit access through
bit indexes `0` through `31`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag32be` values only for integers in the
four-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow four-byte little-endian visible flag bitset slice is implemented as
`Flag32le` for generated binary schema decode and encode helpers. `Flag32le`
consumes and emits four bytes through the existing `UInt32le` representation
path, decodes to the source-visible `Flag32le(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt32le` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in four bytes.
Pure prelude helpers expose checked four-byte `Flag32le` bit access through
bit indexes `0` through `31`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag32le` values only for integers in the
four-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow five-byte big-endian visible flag bitset slice is implemented as
`Flag40be` for generated binary schema decode and encode helpers. `Flag40be`
consumes and emits five bytes through the existing `UInt40be` representation
path, decodes to the source-visible `Flag40be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt40be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in five bytes.
Pure prelude helpers expose checked five-byte `Flag40be` bit access through
bit indexes `0` through `39`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag40be` values only for integers in the
five-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow five-byte little-endian visible flag bitset slice is implemented
as `Flag40le` for generated binary schema decode and encode helpers.
`Flag40le` consumes and emits five bytes through the existing `UInt40le`
representation path, decodes to the source-visible `Flag40le(bits: Int)`
value instead of a raw `Int`, preserves existing `UInt40le` field behavior,
shares exact-width truncation behavior, supports direct mapped-record decode
and encode, and reports existing encode value-representation failures when
`bits` cannot be represented in five bytes.
Pure prelude helpers expose checked five-byte `Flag40le` bit access through
bit indexes `0` through `39`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag40le` values only for integers in the
five-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow six-byte big-endian visible flag bitset slice is implemented as
`Flag48be` for generated binary schema decode and encode helpers. `Flag48be`
consumes and emits six bytes through the existing `UInt48be` representation
path, decodes to the source-visible `Flag48be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt48be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in six bytes.
Pure prelude helpers expose checked six-byte `Flag48be` bit access through
bit indexes `0` through `47`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag48be` values only for integers in the
six-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow six-byte little-endian visible flag bitset slice is implemented as
`Flag48le` for generated binary schema decode and encode helpers. `Flag48le`
consumes and emits six bytes through the existing `UInt48le` representation
path, decodes to the source-visible `Flag48le(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt48le` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in six bytes.
Pure prelude helpers expose checked six-byte `Flag48le` bit access through
bit indexes `0` through `47`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag48le` values only for integers in the
six-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow seven-byte big-endian visible flag bitset slice is implemented as
`Flag56be` for generated binary schema decode and encode helpers. `Flag56be`
consumes and emits seven bytes through the existing `UInt56be` representation
path, decodes to the source-visible `Flag56be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt56be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in seven bytes.
Pure prelude helpers expose checked seven-byte `Flag56be` bit access through
bit indexes `0` through `55`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag56be` values only for integers in the
seven-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow seven-byte little-endian visible flag bitset slice is implemented
as `Flag56le` for generated binary schema decode and encode helpers.
`Flag56le` consumes and emits seven bytes through the existing `UInt56le`
representation path, decodes to the source-visible `Flag56le(bits: Int)`
value instead of a raw `Int`, preserves existing `UInt56le` field behavior,
shares exact-width truncation behavior, supports direct mapped-record decode
and encode, and reports existing encode value-representation failures when
`bits` cannot be represented in seven bytes.
Pure prelude helpers expose checked seven-byte `Flag56le` bit access through
bit indexes `0` through `55`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag56le` values only for integers in the
seven-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow eight-byte big-endian visible flag bitset slice is implemented as
`Flag64be` for generated binary schema decode and encode helpers. `Flag64be`
consumes and emits eight bytes through the existing `UInt64be` representation
path, decodes to the source-visible `Flag64be(bits: Int)` value instead of a
raw `Int`, preserves existing `UInt64be` field behavior, shares exact-width
truncation behavior, supports direct mapped-record decode and encode, and
reports existing encode value-representation failures when `bits` cannot be
represented in eight bytes.
Pure prelude helpers expose checked eight-byte `Flag64be` bit access through
bit indexes `0` through `63`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag64be` values only for integers in the
eight-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow eight-byte little-endian visible flag bitset slice is implemented
as `Flag64le` for generated binary schema decode and encode helpers.
`Flag64le` consumes and emits eight bytes through the existing `UInt64le`
representation path, decodes to the source-visible `Flag64le(bits: Int)`
value instead of a raw `Int`, preserves existing `UInt64le` field behavior,
shares exact-width truncation behavior, supports direct mapped-record decode
and encode, and reports existing encode value-representation failures when
`bits` cannot be represented in eight bytes.
Pure prelude helpers expose checked eight-byte `Flag64le` bit access through
bit indexes `0` through `63`, returning `Result` failures for out-of-range
indexes instead of masking or wrapping. Raw-bit helpers expose the wrapped
integer bits and construct `Flag64le` values only for integers in the
eight-byte range, returning `Result` failures before invalid values reach
generated schema encoders.
The narrow bounded repeated payload slice is implemented as
`Repeat(count_field, Payload)` and
`Repeat(left_count - right_count, Payload)` for generated binary schema decode
and encode helpers. `Repeat(left_count + right_count, Payload)` is also
implemented for generated binary schema decode and encode helpers.
`Repeat(left_count * right_count, Payload)` is implemented for generated
binary schema decode and encode helpers. `Repeat(left_count / right_count,
Payload)` is implemented for generated binary schema decode and encode
helpers.
The count field or count operands must be earlier visible
`Int` fields in the same schema, and the payload must be one of the
implemented byte-aligned exact-width unsigned primitives, an eligible nested
binary schema payload, or `ByteView(length_field)` when the length field is an
earlier visible `Int` field. Primitive repeats decode and encode as
`List<Int>`; nested schema repeats decode and encode as lists of the nested
schema's decoded record shape; repeated byte views decode and encode as
`List<ByteView>`. Negative computed counts report the existing schema
length/count boundary shape, and division by zero reports
`schema.length_division_by_zero`. Encode rejects list length, primitive range,
nested element representation, and repeated byte-view element length
mismatches through `EncodeError`; element failures append an index segment
before nested schema field path segments or at the repeated byte-view element
path.
The completed nested schema payload part of this repeat slice is archived under
`../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md`;
current behavior is specified under `../specification/`.
The generated length-bounded byte payload slice is implemented as
`ByteView(length_field)`, `ByteView(left_length - right_length)`,
`ByteView(left_length + right_length)`,
`ByteView(left_length * right_length)`, and
`ByteView(left_length / right_length)` for generated binary schema decode and
encode helpers. The length operands must be earlier visible `Int` fields in
the same schema, the encoded value record keeps the length operand fields and
the `ByteView` payload field, the helper writes the earlier fields normally
and then writes exactly the bounded bytes from the supplied view, negative
computed decode lengths report
`schema.length_out_of_bounds`, division by zero reports
`schema.length_division_by_zero`, and mismatched encode view counts return the
existing structured `EncodeError` value-representation shape.
The narrow schema-level structural validation slice is implemented as one
`validate` predicate after binary schema fields. Generated decode helpers run
that predicate after all fields and field-local validation have succeeded and
before structural mapping returns the decoded value. The predicate can
reference only decoded `Int` fields in the same schema and reports
`schema.validation_failed` with schema path, predicate text, decoded values,
byte offset, and byte preview details.

## Problem

HTTP/2 frame decoding needs more than ordinary records. A frame header contains
non-byte-aligned semantic fields, endian-sensitive integers, flags, reserved
bits, and a payload whose interpretation depends on a tag value. These are
external representation facts, not internal Veln type declarations.

## Scope

Define remaining binary schema support beyond the implemented
`Http2FrameHeaderWire` generated helper, little-endian primitive widths
through `UInt64le`, the `UInt48be`, `UInt56be`, and `UInt64be` big-endian
primitives, payload-boundary,
closed-dispatch, extension-dispatch, same-module nested dispatch payload,
imported nested dispatch payload decode, mixed closed dispatch selected
mapping decode and encode, and imported nested dispatch payload encode slices
for:

- executable exact-width unsigned field reads and writes beyond the
  implemented primitive helper slices
- endian-aware field reads and writes
- reserved-bit forms beyond the implemented byte-aligned representation-only
  fields, one-byte, two-byte, three-byte, and four-byte packed reserved
  prefixes,
  one-byte, two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte,
  and eight-byte packed reserved suffixes, and
  `ReservedBits(1, 0)` plus `UInt31be` shared-bit layout, and
  non-byte-aligned middle `UIntN` plus `ReservedBits(width, value)` plus
  `UIntN` layouts whose widths complete one byte or one two-byte, three-byte,
  or four-byte big-endian storage unit, one-byte, two-byte, three-byte,
  four-byte, five-byte, six-byte, seven-byte, and eight-byte reserved prefix
  groups followed by two visible `UIntN` fields, including two-byte reserved
  prefix widths one through fourteen, three-byte reserved prefix widths
  seventeen through twenty-three, four-byte reserved prefix widths
  twenty-five through thirty-one, five-byte reserved prefix width
  thirty-three, six-byte reserved prefix width forty-one, seven-byte reserved
  prefix width forty-nine, and eight-byte reserved prefix width fifty-seven,
  and
  consecutive non-byte-aligned
  `UIntN` and `ReservedBits(width, value)` groups that complete one byte or
  one two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte, or
  eight-byte big-endian storage unit
- flag vocabulary beyond the implemented one-byte `Flag8` bitset,
  two-byte big-endian `Flag16be` bitset, two-byte little-endian `Flag16le`
  bitset, four-byte big-endian `Flag32be` bitset, four-byte little-endian
  `Flag32le` bitset, five-byte big-endian `Flag40be` bitset, five-byte
  little-endian `Flag40le` bitset, six-byte big-endian `Flag48be` bitset,
  six-byte little-endian `Flag48le` bitset, seven-byte big-endian `Flag56be`
  bitset, seven-byte little-endian `Flag56le` bitset, eight-byte big-endian
  `Flag64be` bitset, and eight-byte little-endian `Flag64le` bitset, checked
  `Flag8`, `Flag16be`, `Flag16le`, `Flag32be`, `Flag32le`, `Flag40be`,
  `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`, `Flag56le`, `Flag64be`,
  and `Flag64le` bit and raw-bit helper access,
  direct structural mapping boundary, and implemented direct constructor
  mapped encode boundaries, including broader frame-specific ADTs beyond the
  implemented record-payload and nested constructor slices
- general schema-declared length-prefixed payloads beyond the implemented
  `ByteView(length_field)`, `ByteView(left_length - right_length)`,
  `ByteView(left_length + right_length)`,
  `ByteView(left_length * right_length)`, and
  `ByteView(left_length / right_length)` decode and encode helper slices
- field references inside later field definitions beyond implemented bounded
  repeat counts, byte-view lengths, dispatch tags, extension dispatch tags and
  lengths, and their declaration-time missing, forward, and wrong-role
  reference diagnostics
- support rather than rejection for recursive dispatch payload schemas outside
  the selected same-module or public imported length-bounded dispatch
  decode-and-encode slice and dispatch payload schemas outside the generated
  helper slice

## Discussion Result: Dependent Structure Boundary

Binary schemas should support only representation-local dependencies over
fields decoded earlier in the same schema. A prior field may size a later byte
range, select a tagged payload schema, drive fixed or reserved-field
validation, constrain a payload multiple, or participate in mapping into an
independently declared Veln value.

The schema vocabulary should not include general loops, arbitrary function
calls, negotiated settings lookup, connection or stream state access, mutation,
or recovery behavior. Those concerns belong in explicit codec functions,
library codec state, or protocol-core transition functions.

Bounded repeated structures may be considered as schema primitives when their
count or byte length is derived from a prior field and diagnostics remain
field-path and byte-offset based. Unbounded repetition and stateful parsing
must stay outside schema declarations.

## Discussion Result: Exact-Width Primitive Names

The declaration-time source-surface slice for exact-width unsigned names now
lives under `../specification/source-surface.md`. Those names belong to the
binary schema primitive vocabulary as field representation names, not ordinary
source-visible numeric types.
The completed `UInt56be` and `UInt56le` generated-helper slice is recorded in
[Binary Schema UInt56 Primitives](../reference/implemented-proposals/binary-schema-u56-primitives.md);
current behavior is specified by `../specification/source-surface.md`,
`../specification/execution.md`, and checked executable examples.

Remaining primitive work is for widths and forms outside the implemented
exact-width vocabulary. A decoded field should map to `Int` by default, or to
an independently declared Veln record, ADT, or wrapper through an explicit
mapping rule. This keeps schema declarations responsible for byte layout while
keeping ordinary Veln values responsible for protocol meaning.

The implemented narrow executable slices already make `UInt1` through
`UInt8`, `UInt16be`, `UInt24be`, `UInt31be`, `UInt32be`, `UInt40be`,
`UInt48be`, `UInt56be`, and `UInt64be` consume fixed-width unsigned
big-endian fields, and `UInt16le`, `UInt24le`, `UInt31le`, `UInt32le`,
`UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le` consume fixed-width
unsigned little-endian fields,
then return ordinary `Int` values for representable visible fields.
The implemented exact-width primitive encode helper slice emits those visible
ordinary `Int` fields in their declared byte order as `ByteChunk` output and
reports structured `EncodeError` range failures. The implemented reserved-bit
encode slice also
accepts byte-aligned `ReservedBits(width, value)` fields, omits the reserved
field from the encoder value record, and writes the declared fixed value. It
also accepts `ReservedBits(1, 0)` immediately before `UInt31be` and writes
the required zero high bit in the shared four-byte stream identifier
position. The implemented standalone sub-byte primitive slice consumes
`UInt1` through `UInt7` visible fields from one byte each, masks the declared
low bits into ordinary `Int` values, emits one byte per field from accepted
`Int` values, preserves structural mapping and generated decode-step and
derived codec eligibility, and reports existing truncation and
`codec.encode_value_unrepresentable` range-failure shapes. The implemented
closed-dispatch primitive encode slice accepts an earlier visible exact-width
unsigned tag field and exact-width unsigned primitive payload cases, chooses
the payload case from the encoded tag value, and reports structured
`EncodeError` failures for unknown tags or selected payload values outside the
primitive range. The implemented extension-dispatch primitive encode slice
accepts earlier visible exact-width unsigned tag and length fields plus a
`SchemaDispatchPayload<Int>` payload field, writes known primitive payloads,
preserves unknown raw bounded payload bytes, rejects tag or payload variant
disagreements, rejects length fields that do not match the emitted payload byte
count, and reports primitive range failures through structured `EncodeError`
values. The implemented same-module nested dispatch payload encode slice uses
the same earlier-schema eligibility boundary as nested dispatch decode, writes
selected nested records for closed dispatch, writes
`SchemaDispatchPayload::Known` nested records for extension-tolerant dispatch,
keeps unknown extension-tolerant raw payload preservation unchanged, and
reports nested field failures through structured `EncodeError` values. The
implemented imported nested dispatch payload encode slice accepts public
imported payload schemas named through written `use` paths in the same closed
and extension-tolerant encode helper shapes, writes selected imported nested
records through the imported schema helper, keeps unknown extension-tolerant
raw payload preservation unchanged, and reports nested imported field failures
through structured `EncodeError` values. The implemented generalized nested
dispatch payload helper path reuses the generated binary schema helper for
selected nested same-module and imported public payload schemas, including
the supported primitive, reserved-field, fixed-field decode, endian, and
diagnostic behavior already available to ordinary generated schema fields.
The HTTP/2 GOAWAY payload boundary is also implemented as a schema-declared
payload record with `ReservedBits(1, 0)`, `UInt31be`, and `UInt32be` fields,
so outbound GOAWAY payload validation uses the general generated
`byte_encode_<schema>` helper path and preserves schema field-path encode
failures for both visible payload fields.
The completed `Flag40be`, `Flag40le`, `Flag56be`, and `Flag56le` flag bitset
slice is recorded in
[Binary Schema Flag40 And Flag56 Bitsets](../reference/implemented-proposals/binary-schema-flag40-and-flag56-bitsets.md).
The implemented `Flag8` helper slice consumes and emits one-byte visible
bitsets as source-visible `Flag8(bits: Int)` values while leaving existing
`UInt8` fields as ordinary `Int` values. The implemented `Flag16be` helper
slice consumes and emits two-byte big-endian visible bitsets as
source-visible `Flag16be(bits: Int)` values while leaving existing `UInt16be`
fields as ordinary `Int` values. The implemented `Flag16le` helper slice
consumes and emits two-byte little-endian visible bitsets as
source-visible `Flag16le(bits: Int)` values while leaving existing `UInt16le`
fields as ordinary `Int` values. The implemented `Flag32be` helper slice
consumes and emits four-byte big-endian visible bitsets as source-visible
`Flag32be(bits: Int)` values while leaving existing `UInt32be` fields as
ordinary `Int` values. The implemented `Flag32le` helper slice consumes and
emits four-byte little-endian visible bitsets as source-visible
`Flag32le(bits: Int)` values while leaving existing `UInt32le` fields as
ordinary `Int` values. The implemented `Flag40be` helper slice consumes and
emits five-byte big-endian visible bitsets as source-visible
`Flag40be(bits: Int)` values while leaving existing `UInt40be` fields as
ordinary `Int` values. The implemented `Flag40le` helper slice consumes and
emits five-byte little-endian visible bitsets as source-visible
`Flag40le(bits: Int)` values while leaving existing `UInt40le` fields as
ordinary `Int` values. The implemented `Flag48be` helper slice consumes and
emits six-byte big-endian visible bitsets as source-visible
`Flag48be(bits: Int)` values while leaving existing `UInt48be` fields as
ordinary `Int` values. The implemented `Flag48le` helper slice consumes and
emits six-byte little-endian visible bitsets as source-visible
`Flag48le(bits: Int)` values while leaving existing `UInt48le` fields as
ordinary `Int` values. The implemented `Flag56be` helper slice consumes and
emits seven-byte big-endian visible bitsets as source-visible
`Flag56be(bits: Int)` values while leaving existing `UInt56be` fields as
ordinary `Int` values. The implemented `Flag56le` helper slice consumes and
emits seven-byte little-endian visible bitsets as source-visible
`Flag56le(bits: Int)` values while leaving existing `UInt56le` fields as
ordinary `Int` values. The implemented `Flag64be` helper slice consumes and
emits eight-byte big-endian visible bitsets as source-visible
`Flag64be(bits: Int)` values while leaving existing `UInt64be` fields as
ordinary `Int` values. The implemented `Flag64le` helper slice consumes and
emits eight-byte little-endian visible bitsets as source-visible
`Flag64le(bits: Int)` values while leaving existing `UInt64le` fields as
ordinary `Int` values. Structural decode mappings can use decoded
flag values through the implemented field reference, same-module ADT
constructor, pure same-module converter, and imported public pure converter
expression forms where those forms are implemented for the flag type.
Mapped-record encode is implemented when every visible encode field can be
projected back to a schema-local field by a projectable direct field,
record-shaped, field-selection, same-module pure converter-call mapping with
an explicitly named same-module pure inverse converter, or imported public
pure converter-call mapping with an explicitly named imported public pure
inverse converter through written import paths. The completed narrow arithmetic
mapped encode slice is archived under
[Binary Schema Mapping Arithmetic Encode](../reference/implemented-proposals/binary-schema-mapping-arithmetic-encode.md).
A single target
field assigned from a direct ADT constructor call is also implemented when
every constructor payload argument is a schema-local field supported by the
generated encode helper, including the single-payload flag and exact-width
integer cases, the first multi-payload direct-field case, and one single
record payload whose fields are direct schema-local visible field references
supported by the generated encode helper. Constructor payload arguments can
also be nested ADT constructor calls when their leaves stay within those
projectable forms. General inverse mapping for imported converter calls
without explicit written import paths, selected mappings outside the
implemented direct-field selected mapping slices, and other non-direct
expressions remains outside the implemented encode slice.
The implemented bounded repeated helper slice consumes and emits
`Repeat(count_field, Payload)` fields when `count_field` names an earlier
visible `Int` field, `Repeat(left_count - right_count, Payload)` fields when
both operands name earlier visible `Int` fields in the same schema, and
`Repeat(left_count + right_count, Payload)`,
`Repeat(left_count * right_count, Payload)`, and
`Repeat(left_count / right_count, Payload)` fields when both operands name
earlier visible `Int` fields in the same schema.
`Payload` is `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`,
`UInt31be`, `UInt31le`, `UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`,
`UInt48be`, `UInt48le`, `UInt56be`, `UInt56le`, `UInt64be`, `UInt64le`, or an
eligible same-module or public imported nested binary schema payload named
through a written `use` path, including when that nested schema contains
`ByteView(length_field)` whose length field is an earlier visible `Int` field
in the same nested schema, plus `ByteView(length_field)` when the repeat
payload length field is an earlier visible `Int` field.
General schema-owned decode and encode beyond the implemented slices, support
rather than rejection for recursive dispatch payload schemas outside the
selected same-module or public imported length-bounded dispatch
decode-and-encode slice, dispatch payload schemas outside the generated helper
slice beyond the implemented nested generated-helper vocabulary, and mapping
beyond the implemented slices remain proposal work.
A `UInt31be` field
represents the 31-bit unsigned value in a big-endian field position whose
remaining bit is handled as a reserved or fixed schema bit. The 31-bit value
should not become a general-purpose source type.

## Discussion Result: Reserved Bit Spelling

Reserved bits are spelled as schema-local fixed fields that are consumed from
the external representation but omitted from the mapped Veln value by default.
The byte-aligned `ReservedBits(width, value)` slice, one-byte, two-byte,
three-byte, and four-byte packed reserved prefix slices, one-byte, two-byte,
three-byte, four-byte, five-byte, six-byte, seven-byte, and eight-byte packed
reserved suffix slice, and the
`ReservedBits(1, 0)` plus `UInt31be` shared-bit layout, and middle
`UIntN` plus `ReservedBits(width, value)` plus `UIntN` layouts whose widths
complete one byte or one two-byte, three-byte, or four-byte big-endian
storage unit, and one-byte, two-byte, three-byte, four-byte, five-byte,
six-byte, seven-byte, and eight-byte
reserved prefix groups followed by two visible `UIntN` fields, including
two-byte reserved prefix widths one through fourteen, three-byte reserved
prefix widths seventeen through twenty-three, four-byte reserved prefix widths
twenty-five through thirty-one, five-byte reserved prefix width
thirty-three, six-byte reserved prefix width forty-one, seven-byte reserved
prefix width forty-nine, and eight-byte reserved prefix width fifty-seven,
the narrow
two-byte byte-interleaved middle
layout with a sub-byte visible field, a sub-byte reserved field, `UInt8`, and
a final sub-byte visible field, and consecutive non-byte-aligned
`UIntN` and `ReservedBits(width, value)` groups that complete one byte or one
two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte, or eight-byte
big-endian storage unit, plus the narrow `ReservedBits(9, 0)` plus `UInt8`
byte-prefix layout, and the narrow two-byte suffix group where two visible
`UIntN` fields, the second one `UInt8`, are followed by a non-byte-aligned
`ReservedBits(width, value)` suffix, are implemented under
`../specification/execution.md`. Completed split reserved group history is
recorded in
[Binary Schema Split Reserved Groups](../reference/implemented-proposals/binary-schema-split-reserved-groups.md),
with focused seven-byte and eight-byte companion records in
[Binary Schema Seven-Byte Split Reserved Layouts](../reference/implemented-proposals/binary-schema-seven-byte-split-reserved-layouts.md)
and
[Binary Schema Eight-Byte Split Reserved Layouts](../reference/implemented-proposals/binary-schema-eight-byte-split-reserved-layouts.md).
The completed seven-byte and eight-byte reserved prefix group slice is recorded
in
[Binary Schema Wide Reserved Prefix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-prefix-groups.md).
The completed seven-byte and eight-byte reserved suffix slice is recorded in
[Binary Schema Wide Reserved Suffix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-suffix-groups.md).
The completed two-byte suffix reserved group slice is recorded in
[Binary Schema Suffix Reserved Groups](../reference/implemented-proposals/binary-schema-suffix-reserved-groups.md).
The completed `ReservedBits(15, value)` plus `UInt1` two-field boundary is
recorded in
[Binary Schema Reserved Fifteen-Bit Prefix](../reference/implemented-proposals/binary-schema-reserved-fifteen-bit-prefix.md).
Remaining proposal work is limited to non-byte-aligned shapes outside those
layouts and any later opt-in mapping exposure.

Use a `ReservedBits(width, value)` binary schema primitive for this purpose.
The field still has a schema-local name so diagnostics can report a stable
field path, but the primitive marks the field as representation-only so it is
not mapped into the produced Veln record or ADT unless a later explicit mapping
rule opts in.

For HTTP/2, the stream identifier field is therefore written as a one-bit
reserved field followed by the visible 31-bit value:

```text
schema Http2FrameHeader
  format binary

  length: UInt24be
  kind: UInt8 as FrameKind
  flags: UInt8
  stream_reserved: ReservedBits(1, 0)
  stream_id: UInt31be
end
```

`ReservedBits` is only for representation bits whose required value is fixed
by the external format and whose decoded value is not semantically meaningful
to the program. Visible flags, extension bits, and protocol values should use
ordinary fields with validation or mapping rules instead.

Invalid reserved bits are schema structural failures. Diagnostics should point
at the reserved field path and byte offset, report the expected bit pattern and
actual bit pattern, and keep protocol-state causes out of the primary schema
failure.

## Discussion Result: Unknown Dispatch Preservation

Tag dispatch should preserve unknown tags when the schema author explicitly
marks the dispatch as extension-tolerant. The decoded value should keep the raw
tag value, the already validated header fields that are part of the enclosing
schema, and the bounded payload bytes selected by the length field. Unknown
payload bytes are opaque; schemas should not invent a partial internal shape
for a tag whose representation is not declared.

Unknown dispatch is not an error by itself in an extension-tolerant schema.
Errors still come from structural facts the schema owns, such as truncated
input, invalid fixed or reserved fields, a length that cannot be sliced from
the buffered bytes, or an unknown tag in a closed dispatch. Protocol code can
then choose whether to ignore the unknown value, surface it to callers, or
reject it because of protocol state.

This keeps binary schemas useful for extensible protocols without making the
codec silently discard bytes that fixtures, diagnostics, forwarding, or later
extension handling may need. The retained payload must remain bounded by the
decoded length field so extension preservation cannot keep unrelated consumed
input alive.

The implemented narrow slice exposes this through
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)`, where the
tag and length fields must already be decoded in the same schema, known cases
use implemented exact-width unsigned primitive payloads, same-module nested
binary schema payloads, or public imported nested binary schema payloads, and
unknown cases retain the bounded raw `ByteView`. Recursive or otherwise
ineligible nested payload schemas outside the selected same-module or public
imported length-bounded dispatch boundary and protocol-state legality checks
remain outside this slice.

## Discussion Result: Binary Schema Value Mapping

Binary schema values should use the structural mapping rule from the schema
declaration surface: a schema maps validated schema-local fields into an
independently declared record or ADT constructor through an explicit mapping
clause.

Exact-width integer primitives produce ordinary `Int` values unless a
schema-declared representation conversion maps the field into a visible domain
type. `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`, `Flag32be`,
`Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`,
`Flag56le`, `Flag64be`, and `Flag64le` fields produce source-visible
`Flag8(bits: Int)`, `Flag16be(bits: Int)`, `Flag16le(bits: Int)`,
`Flag24be(bits: Int)`, `Flag24le(bits: Int)`, `Flag32be(bits: Int)`,
`Flag32le(bits: Int)`, `Flag40be(bits: Int)`, `Flag40le(bits: Int)`,
`Flag48be(bits: Int)`, `Flag48le(bits: Int)`, `Flag56be(bits: Int)`,
`Flag56le(bits: Int)`, `Flag64be(bits: Int)`, and `Flag64le(bits: Int)`
values in the implemented helper and mapping slices. Byte ranges produce `ByteView` or
`ByteChunk` values according to the field vocabulary. Reserved fields, fixed
fields, and other representation-only fields stay available for validation
and diagnostics but are omitted from the mapped value unless the mapping
explicitly includes them.

Tag dispatch maps known cases to explicit target constructors or records. An
extension-tolerant unknown case must map to a target shape that can carry the
raw tag value and bounded payload bytes, such as an `Unknown` constructor. A
closed dispatch has no unknown mapping; an unrecognized tag remains a schema
structural failure.

The mapping checker should reject missing target fields, duplicate target
assignments, unknown constructors, and assignments whose schema-local value
type does not match the target field. It should not run arbitrary source
functions or consult protocol state; those conversions belong in explicit
codec functions after schema decoding.

## Discussion Result: Field Reference Scope

Field references inside binary schemas should be schema-local, unique, and
backward-only. A field definition may reference fields decoded earlier in the
same schema by their field name. It must not reference later fields, ordinary
source values, imported functions, runtime settings, connection state, or
stream state.

Schema field names are unique within the field scope that declares them.
Shadowing is rejected so every field path used by diagnostics remains stable.
Dispatch cases and nested payload schemas do not implicitly capture outer
fields; any shared value must be passed through an explicit schema field or
context parameter accepted by a later syntax proposal.

References denote the validated schema value of the earlier field, not the raw
bytes that produced it. Type checking uses the role required by the consuming
primitive: byte lengths and consumed counts require a `ByteCount` or a
non-negative integer field with a checked conversion; dispatch tags require a
field whose ordinary decoded value can be compared with the declared case tag;
fixed and reserved-field validation may only compare compatible integer or bit
patterns.

A failed reference is a schema declaration error, not a codec failure at input
time. Diagnostics should report the reference span, name the missing,
forward, or wrong-typed field, and include the candidate field path when the
author likely referred to an earlier field with a compatible role.

## Non-Goals

- Do not encode HTTP/2 stream-state legality in schema declarations.
- Do not require HPACK support.
- Do not define network effects or task scheduling.
- Do not optimize binary layout.

## Remaining Completion Criteria

- Broader unsupported field layouts, other ineligible dispatch payload schemas
  beyond the checked unsupported `ReservedBits`, unsupported `ByteView` length
  references, mapped encode projection diagnostics, and imported recursive
  diagnostics, and schema value mapping beyond the implemented structural,
  constructor field-selection, and mapped-payload eligibility diagnostic
  slices remain proposal work.
