# Execution Boundary

This file specifies the implemented execution boundary.

## Core And IR

Checked core is produced only after semantic diagnostics have no errors. Typed
IR is produced only when checked core is complete. Reachable holes, missing
expressions, constructor arity gaps, call arity gaps, and recognized
concurrency calls block executable IR. For selected `run` and `test` entries,
reachability includes direct function calls, bare and `use` alias-qualified
function declaration values used inside reachable expressions, and function
calls in reachable contract predicates. Reachability also follows bare and
`use` alias-qualified function declaration values passed as contract call
arguments. Calls through a function-typed local binding or parameter are
conservative: when the surface graph does not identify one concrete function
declaration, reachability includes visible function declarations with the same
argument count. In a named source module, a bare function reference resolves
reachability only to functions owned by that same source module. Qualified
calls and function values resolved through selected-file `use` aliases keep
the imported module identity, so same-named functions from other modules are
not included only because their local name matches. Bare local bindings,
parameters, and match-pattern bindings shadow same-named function declarations
for selected-entry reachability; a shadowed bare name is treated as the local
value, not as a function declaration value. The implemented execution fixtures
cover function declarations used as function-typed values, function-typed value
calls, opaque function-typed value call reachability, contract helper
reachability, contract function value reachability, imported-call
reachable-hole blocking, selected-entry reachable-hole blocking, local
shadowing of function declarations, and selected-entry concurrency blockers
before JVM execution.
When a function or test body omits the final expression line, checked core and
typed IR materialize that omission as an explicit `()` return.

The typed IR is runtime-neutral. JVM class names, Java method names, boxed
runtime representation, generated artifact paths, cache keys, and runtime
helper layout are backend details and are not language facts.

Stdio operations are serialized at the runtime handler boundary. Each
`stdio::print`, `stdio::println`, `stdio::eprint`, and `stdio::eprintln`
operation writes its complete logical output and records its test event while
holding the same handler lock. Captured event `sequence` values therefore
define one total operation order across stdout and stderr for a selected run or
test case, including calls made by spawned tasks.

## JVM Backend

The JVM backend emits classfile artifacts directly for the implemented IR
subset:

- functions, parameters, locals, expression statements, and returns
- omitted tail expressions as `()` returns
- literals, records, vecs, `Ok`, `Err`, `Some`, `None`, their `Result::` or
  `Option::` qualified forms, and `?`
- `match` expressions over literals, `_`, bindings, and built-in `Option` and
  `Result` constructors, after finite-domain exhaustiveness diagnostics have
  passed
- record field access
- stdio builtins, prelude helpers, ordinary function calls, and function-value
  calls
- file-system, network, time, and current-process standard library intrinsics
- bounded channel construction, sender clone, send, receive, and close calls
- two-receiver channel selection calls with optional timeout
- task spawn, join, and cancellation calls
- pipelines with named or qualified call targets lowered to calls with the
  left expression inserted as the first argument
- runtime `require` checks at function entry and runtime `ensure` checks before
  tail-expression returns and `?` early returns
- integer and boolean operators used by the implemented type rules

Generated runtime helpers may use mutable builders while constructing records,
vecs, and dictionary update results. Values returned to Veln user code are
frozen at that boundary: records and dictionaries are exposed as unmodifiable
maps, vecs are exposed as unmodifiable host lists, and prelude container updates
return new frozen containers instead of mutating the input value in place.
Standard `List` helper traversals, including `list_fold`, `list_reverse`,
`list_map`, `list_filter`, and `list_try_map`, execute through runtime support
that iterates over the list representation instead of growing the host call
stack. This support does not expose source-level tail-call syntax or a general
tail-call optimization guarantee.

Standard byte chunk and byte view helpers are pure prelude runtime operations.
The runtime constructs immutable `ByteChunk` values, computes `ByteCount`,
appends chunks without mutating inputs, decodes compact ASCII hex fixture
text, constructs bounded `ByteView` values, materializes bounded views as
chunks, derives bounded views within existing views, constructs outgoing
`List<ByteChunk>` values, and reads or writes fixed-width unsigned big-endian
and little-endian integer representations. These helpers return `Result`
failures for invalid byte values, invalid hex fixture text, negative counts or
offsets, slice or drop counts that exceed the chunk or view length, view ranges
that exceed the chunk or view length, truncated reads, and fixed-width unsigned
conversion overflow. Hex fixture decoding accepts ASCII hex byte pairs with
ASCII whitespace between complete bytes only; invalid characters and dangling
nibbles return stable fixture hex error ids with decoded byte offset and nibble
position in the error text. When such a failure propagates out of a
`run --json` entry as an `Err(String)`, the
result failure details also include the fixture text span, decoded
`ByteOffset`, nibble position, and nearby fixture text context. Byte views
cross task and channel freeze boundaries with the same bounded bytes, logical
`ByteOffset`, and `ByteCount` observed by the sender; the checked
`examples/specification/run/binary-byteview-freeze-boundary/` case observes
those facts after the original buffered input has advanced. The runtime may
copy, share, pin, or otherwise preserve the bounded bytes, but source behavior
does not expose a memory layout or zero-copy guarantee. `byte_view_to_chunk`
exposes owned-byte semantics directly by returning an immutable `ByteChunk`
containing exactly the bounded view bytes. The exact host representation of
byte chunks, byte views, counts, offsets, and bytes is backend-owned.

The generated `Http2FrameHeaderWire` binary schema helper decodes the HTTP/2
frame-header field sequence over `ByteView`. It consumes a `UInt24be` length
field, two `UInt8` fields, one `ReservedBits(1, 0)` field, and one `UInt31be`
stream id field. Exact-width unsigned fields produce ordinary `Int` values in
the decoded record. The reserved field is representation-only: it advances the
decode position and validates the fixed bit pattern but is omitted from the
record. Truncated schema fields return a `schema.truncated_field` result
failure with expected and available byte counts. Reserved-bit mismatches
return `schema.reserved_bits_mismatch` with bit width, expected value, actual
value, structured byte preview fields, byte offset, and schema field path.

The `SchemaWidthSample` primitive decode helper consumes one `UInt16be` field
followed by one `UInt32be` field from a `ByteView`. Both fields produce
ordinary `Int` values in the decoded record. Generated binary schema decode
helpers also support `UInt16le`, `UInt24le`, `UInt31le`, `UInt32le`,
`UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le` as little-endian unsigned
primitives.
`UInt40be` is implemented as the matching five-byte big-endian primitive,
`UInt48be` as the matching six-byte big-endian primitive, `UInt56be` as the
matching seven-byte big-endian primitive, and `UInt64be` and `UInt64le` are
implemented as eight-byte schema primitives and decode to ordinary `Int`
values when the decoded value is representable as source-visible `Int`. The
helper preserves structural
`map to` runtime mappings. Truncation reports the same `schema.truncated_field`
diagnostic shape as the frame-header helper, including byte offset,
structured field path, expected byte count, available byte count, readiness,
and structured byte preview fields.
Generated binary schema decode helpers also support byte-aligned
`ReservedBits(width, value)` fields up to four bytes wide as
representation-only fields. The helper consumes the reserved bytes in
declaration order, validates the declared fixed value, omits the field from
the decoded value and structural mapping source values, and reports
`schema.truncated_field` or `schema.reserved_bits_mismatch` at the reserved
field path when the input is short or the fixed value differs.
Generated binary schema decode helpers also support packed reserved prefixes:
`ReservedBits(width, value)` where `width` is one through seven may be
followed by the visible `UIntN` primitive whose width completes the byte,
widths nine through fifteen may be followed by the visible `UIntN` primitive
whose width completes the same two-byte big-endian storage unit, and widths
seventeen through twenty-three may be followed by the visible `UIntN`
primitive whose width completes the same three-byte big-endian storage unit,
and widths twenty-five through thirty-one may be followed by the visible
`UIntN` primitive whose width completes the same four-byte big-endian storage
unit. A supported prefix group may also place `ReservedBits(width, value)`
before two visible sub-byte or byte-width `UIntN` fields when all three widths
complete one byte, a two-byte big-endian storage unit, a three-byte
big-endian storage unit, or a four-byte big-endian storage unit. In the
two-byte form, reserved prefix widths one
through fourteen are accepted when the two visible fields complete the
remaining bits in declaration order; in the three-byte form, reserved prefix
widths seventeen through twenty-three are accepted when the two visible fields
complete the remaining bits in declaration order; in the four-byte form,
reserved prefix widths twenty-five through thirty-one are accepted when the
two visible fields complete the remaining bits in declaration order.
The helper validates the high reserved bits, decodes the following visible
bits from their declared high-to-low positions as ordinary `Int` values,
omits the reserved field from decoded records and mapping source values, and
advances by the shared storage width for the pair or group. The inverse
suffix layout is also supported: a visible `UIntN` field followed immediately
by `ReservedBits(width, value)` where the two widths complete one byte or the
same two-byte, three-byte, or four-byte big-endian storage unit.
That form decodes the visible value from the high bits, validates the low
reserved bits at the reserved field path, omits the reserved field, and
advances by the shared storage width. The supported middle layout is a
visible `UIntN` field, a `ReservedBits(width, value)` field, and another
visible `UIntN` field whose widths together complete one byte or the same
two-byte, three-byte, or four-byte big-endian storage unit. That form decodes
the visible fields from their declared high-to-low positions, validates the
middle reserved field at the reserved field path, omits the reserved field,
and advances by the shared storage width. The narrow two-byte interleaved
form also accepts a sub-byte visible `UIntN` field, a sub-byte middle
`ReservedBits(width, value)` field, a byte-width visible `UInt8` field, and a
final sub-byte visible `UIntN` field when the four widths complete the same
two-byte big-endian storage unit without completing a storage byte before the
`UInt8` field. The same shared-storage rule also
covers consecutive non-byte-aligned `UIntN` and
`ReservedBits(width, value)` fields when the group contains at least one
visible field and at least one reserved field, every visible field is a
big-endian sub-byte `UIntN`, and the declared widths complete one byte or the
same two-byte, three-byte, or four-byte big-endian storage unit. Reserved
fields in the group remain representation-only, each reserved value is
validated at its own field path, and visible fields are decoded from their
declared high-to-low positions.
Generated binary schema decode helpers also support standalone visible
`UInt1` through `UInt7` fields. Each field consumes one byte, exposes the
declared low bits as an ordinary `Int`, advances by one byte, preserves
structural `map to` runtime mappings, and uses the same truncation diagnostic
shape as other exact-width primitives.
Generated binary schema decode helpers also support bounded
`Repeat(count_field, Payload)` fields when `count_field` is an earlier
visible exact-width unsigned field decoded as `Int` and `Payload` is either
an implemented byte-aligned exact-width unsigned primitive or an eligible
nested binary schema payload. `Repeat(left_count - right_count, Payload)`
and `Repeat(left_count + right_count, Payload)` use the difference or sum of
two earlier visible exact-width unsigned `Int` fields as the repeat count.
Repeated primitive fields decode as `List<Int>`; repeated
nested schema fields decode as lists of the nested schema's decoded record
shape. The helper reads exactly the computed count in declaration order.
Negative computed counts report `schema.length_out_of_bounds` at the repeat
field path. Element failures keep the repeated field path and append an
`index` segment before nested schema field segments.
Generated binary schema decode helpers also treat a field-local equality
predicate of the form `field == literal` or `literal == field` as a visible
schema-owned fixed field when the literal fits the field's external integer
range. Matching values remain visible in the decoded result. A mismatch
reports `schema.fixed_field_mismatch` at the field byte offset with schema
field path, expected value, actual value, and structured byte preview fields.

The binary schema field-local validation execution slice decodes fields in
declaration order for generated `byte_decode_<schema>` helpers when every
field uses an implemented exact-width unsigned binary primitive. It checks a
supported `where` predicate after the owning field is decoded. Predicate
evaluation may read the current field and earlier decoded fields and supports
comparison, boolean, literal, arithmetic, prefix `not`, and grouping forms.
Later-field references, unknown fields, and ordinary source bindings named by
a predicate return an unsupported schema predicate reference error. Passing
validation returns ordinary `Int` values for decoded fields unless the schema
has one eligible structural `map to Target` clause. In that mapped slice, the
generated helper constructs the target record field names from decoded schema
fields, record construction expressions, and ADT constructor construction
expressions after all field-local `where` predicates pass. A mapping
assignment may also use `+`, `-`, `*`, and `/` over decoded schema-local
`Int` fields, integer literals, `Int`-returning converter calls, and nested
supported mapping arithmetic expressions. Division by zero returns
`schema.mapping_division_by_zero` at the offset after the decoded schema body
with the schema and target-field path. A mapping assignment may also call one pure same-module converter
function or one imported public pure converter function through a written
`use` path or alias with one, two, or three arguments. Each argument must be
either one decoded schema-local field or an already implemented structural
mapping expression made from decoded schema fields, records, ADT constructors,
integer arithmetic mapping expressions, and nested combinations of those forms
before assigning the returned value to the target field. Multiple selected mappings
may use narrow boolean selector expressions over decoded schema-local `Int`
fields, integer literals, `==`, `!=`, `and`, `or`, and `not` when the selector
clauses do not overlap and all branches resolve to one target record shape. A
mapping assignment may also select a field from an already supported
structural mapping expression after that source expression is available, when
the source expression has a record-shaped type with the selected field.
Mapping diagnostics reject unknown source fields,
unknown target fields, duplicate or missing target fields, unsupported
expression forms, unresolved constructors or converters, private imported
converters, constructor or converter arity mismatches, impure converters,
converter input or return types that do not match the argument or target
field, and expression types that do not match their target fields or
constructor payload fields before execution. Failed
validation returns `schema.validation_failed` at the owning field byte offset
with structured field path, predicate text, owning field value, decoded values,
and structured byte preview fields.

The same eligible generated binary schema slice also exposes
`byte_decode_step_<schema>` helpers. A decode-step helper receives the bounded
`ByteView` to inspect and an explicit base `ByteOffset` for the first byte in
that view. If the view contains the full exact-width field sequence, the
helper returns `Decoded(value, consumed)` where `value` has the same schema or
mapped record shape as `byte_decode_<schema>` and `consumed` is exactly the
schema byte width. If the open view is shorter than that width, the helper
returns `NeedMore(NeedBytes(count))`, where `count` is the minimum buffered
byte count required before retrying, and it consumes no bytes. This
incremental helper does not change the closed-input `Result` helper path:
closed truncation still reports `schema.truncated_field` through
`byte_decode_<schema>`.

A codec declaration with a valid `derive decode` clause for the same eligible
generated binary schema decode-step slice exposes the codec item name as an
executable decode boundary in ordinary source calls, including supported
middle reserved-bit layouts, same-module nested dispatch payload schemas,
same-module recursive closed and extension dispatch payload schemas,
repeat-backed schemas, and multiple decoded-field selected schema mappings
already accepted by `byte_decode_step_<schema>`. The call accepts the bounded
`ByteView` and explicit base `ByteOffset` and returns the same
`DecodeStep<T>` value as
`byte_decode_step_<schema>`, including mapped record values. `Decoded` reports
the exact consumed byte count; `NeedMore` and `Invalid` consume no bytes. For
the implemented structural mapping slice, `T` is the mapping target record
shape when each assignment source has the same implemented decoded field type
as the target field and all selected mappings resolve to that same record
shape.
A `derive decode` clause is rejected with `codec.derive_helper_unsupported`
when the referenced schema cannot expose the required generated decode-step
helper.

A codec declaration with a valid hand-written `decode with function_name`
clause also exposes the codec item name as an executable decode boundary in
ordinary source calls. The call accepts the bounded `ByteView` and explicit
base `ByteOffset` and invokes the already-checked same-module decode function.
`NeedMore(readiness)` and `Invalid(error)` return unchanged.
`Decoded(value, consumed)` returns unchanged when `consumed` is within the
supplied view length; when `consumed` is outside the supplied view, the codec
boundary returns `Invalid(DecodeError("codec.consumed_count_invalid",
base_offset, codec_name))`. When the referenced schema uses multiple
decoded-field selected mappings that resolve to one implemented target record
shape, the referenced function must return `DecodeStep<T>` for that selected
mapping record shape. Same-module private decode codecs are callable only
inside their declaring module; imported calls require a written qualified
module path to a `pub codec`.

For `veln run` entries, a returned
`DecodeStep::Invalid(DecodeError(id, byte_offset, field_path))` is projected
to a focused human runtime diagnostic and `details.byte_diagnostic` JSON using
the contained diagnostic id, byte offset, and field path, including the
`codec.consumed_count_invalid` result produced by the hand-written codec
boundary when a decoded consumed count is outside the supplied `ByteView`. A
returned
`DecodeStep::NeedMore(readiness)` is projected at the closed-input reporting
boundary as `codec.incomplete_input`, with readiness and requested byte count
details from the source-visible `DecodeReadiness` value. `Decoded` remains an
ordinary successful entry value. The checked examples are
`examples/specification/run/codec-decode-consumed-count-invalid-human/`,
`examples/specification/run/codec-decode-consumed-count-invalid-json/`,
`examples/specification/run/codec-decode-invalid-step-human/`,
`examples/specification/run/codec-decode-invalid-step-json/`,
`examples/specification/run/codec-decode-need-more-human/`, and
`examples/specification/run/codec-decode-need-more-json/`.

A codec declaration with a valid hand-written `encode with function_name`
clause exposes the codec item name as an executable encode boundary in
ordinary source calls. The call invokes the already-checked same-module encode
function with that function's parameters and returns its
`EncodeStep<TState>` value unchanged, including `Encoded`, `Partial`, and
`Invalid` results. A checked budgeted encode example observes `Partial` with
its emitted chunk list, produced byte count, and resumed encoder state as
ordinary source-visible values, then uses the returned state to complete a
later encode call. For the implemented single structural `map to Target`
schema slice, the first encoder parameter remains the mapped target record
shape. Same-module private encode codecs are callable only inside their
declaring module; imported calls require a written qualified module path to a
`pub codec`.

A codec declaration with a valid `derive encode` clause for the same eligible
generated binary schema encode helper slice exposes the codec item name as an
executable encode boundary in ordinary source calls, including eligible nested
dispatch payload schemas, same-module recursive closed and extension dispatch
payload schemas, repeat-backed schemas, direct structural mappings, and
selected structural mappings already accepted by `byte_encode_<schema>`.
The call accepts the generated helper's schema-local value record or mapped
target record, invokes the generated schema encode helper, and returns
`EncodeStep<()>`. Successful helper output is projected from `Ok(ByteChunk)`
to `Encoded(List<ByteChunk>)` with one immutable output chunk. Helper
`Err(EncodeError)` output is projected to `Invalid(EncodeError)`.
Same-module private derived encode codecs are callable only inside their
declaring module; imported calls require a written qualified module path to a
`pub codec`. General generated encode helper behavior outside the exact-width
primitive, supported reserved-bit, length-bounded `ByteView`, bounded repeated
primitive, nested schema, and `ByteView(length_field)` payloads, closed
dispatch, extension dispatch, implemented direct structural mapping, and
eligible nested and same-module recursive dispatch payload slices remains
unimplemented. When a
mapped schema uses a mapping expression shape that cannot be projected back to
the schema-local encode record, the `derive encode` clause is rejected with
`codec.derive_helper_unsupported`.

Eligible generated binary schema encode helpers named
`byte_encode_<schema>` accept one record whose fields match the schema-local
visible exact-width unsigned primitive fields as ordinary `Int` values. For
one structural `map to Target` clause whose assignments project every visible
encode field through direct schema-local field references, record expressions
whose fields are direct schema-local visible field references, field selection
from those record expressions when the selected field maps directly to one
schema-local visible field, or the supported direct ADT constructor wrapper
forms, the helper accepts the mapping target record shape instead and projects
those target fields back to the schema-local encode record before writing
bytes. For multiple selected `map to Target when field == literal` or
`map to Target when field != literal` clauses, the helper accepts the mapping
target record shape when all selected mappings resolve to that same record
shape and every schema-local encode field, including the selector field,
projects back from the selected target record through direct source-field
assignments. The helper selects the mapping whose projected selector value
satisfies the clause and then writes the projected schema-local record.
Length-bounded `ByteView(length_field)`,
`ByteView(left_length - right_length)`,
`ByteView(left_length + right_length)`, and
`ByteView(left_length * right_length)` payload fields are `ByteView` record
fields and emit exactly the bounded bytes from that view after the earlier
visible length operand fields are written. Decode computes arithmetic lengths
from earlier decoded field values and rejects negative or unavailable payload
ranges as `schema.length_out_of_bounds`. If the supplied view count differs
from the earlier length field or computed length expression, the helper returns
`Err(EncodeError("codec.encode_value_unrepresentable", field_path, reason))`
without emitting partial output. Command-facing diagnostics for this
schema-facing conversion boundary preserve the schema field path, expected
count, actual `ByteView` count, length expression, byte offset, bounded byte
preview, and count mismatch reason in human and JSON output. The checked
examples are
`examples/specification/run/binary-schema-byteview-add-decode/`,
`examples/specification/run/binary-schema-byteview-add-truncated-json/`,
`examples/specification/run/binary-schema-byteview-add-encode/`,
`examples/specification/run/binary-schema-byteview-add-encode-length-mismatch/`,
`examples/specification/run/binary-schema-byteview-product-decode/`,
`examples/specification/run/binary-schema-byteview-product-truncated-json/`,
`examples/specification/run/binary-schema-byteview-product-encode/`,
`examples/specification/run/binary-schema-byteview-product-encode-length-mismatch/`,
`examples/specification/run/binary-schema-byteview-encode-diagnostic-json/`
and
`examples/specification/run/binary-schema-byteview-encode-diagnostic-human/`.
A
bounded `Repeat(count_field, Payload)` field emits exactly the number of
elements named by the earlier count field, and
`Repeat(left_count - right_count, Payload)` and
`Repeat(left_count + right_count, Payload)` emit exactly the computed
difference or sum. Primitive payloads use `List<Int>`; nested schema payloads
use a list of the nested schema's decoded record shape; repeated
`ByteView(length_field)` payloads use `List<ByteView>` and write each
element's bounded bytes in declaration order. A list length mismatch,
primitive range failure, repeated byte-view element count mismatch, or nested
element representation failure returns
`Err(EncodeError("codec.encode_value_unrepresentable", field_path, reason))`;
repeated byte-view element failures append the element index to the repeated
field path, and nested element failures prefix the nested schema field path
with the repeated field and element index. A
byte-aligned `ReservedBits(width, value)` field is representation-only: it is
omitted from the record and the helper emits the declared fixed value in
declaration order. A `ReservedBits(1, 0)` field immediately before a
`UInt31be` field keeps the shared stream-identifier layout: it is omitted from
the record and the helper emits the required zero high bit in the shared
four-byte position.
Closed `Dispatch(tag_field, tag => Payload, ...)` fields are eligible when
`tag_field` names an earlier visible exact-width unsigned field and every case
payload is an implemented exact-width unsigned primitive payload or an
eligible nested binary schema payload named as an earlier same-module binary
schema or a public imported binary schema through a written `use` path. The
record contains the visible tag field and
one payload field; nested schema payload fields use the selected nested schema
decoded record shape. The helper chooses the case from the encoded tag value,
writes the selected payload in declaration order, and reports
`codec.dispatch_unknown_tag` when the tag value has no case.
Extension-tolerant
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields are
eligible for the same exact-width unsigned primitive or eligible nested binary
schema payload cases when both the tag and length fields are earlier visible
exact-width unsigned fields. The payload record field is
`SchemaDispatchPayload<T>`, where `T` is the selected primitive `Int` or
nested schema decoded record shape. `Known(value)` writes
the payload selected by the visible tag field. `Unknown(tag, payload)` writes
the bounded raw bytes from the `ByteView` only when the visible tag value is
not a known case and matches the unknown payload tag. The supplied length
field remains explicit: the
helper rejects values whose encoded payload byte count differs from the
earlier length field with `codec.dispatch_length_mismatch`. Visible tag and
payload variant disagreements report `codec.dispatch_mismatch`.
The helper writes fields in declaration order into one immutable `ByteChunk`,
using each primitive's declared byte order, and returns
`Result<ByteChunk, EncodeError>`. `UInt16le`, `UInt24le`, `UInt31le`,
`UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le` emit
little-endian bytes and use the same representability boundaries as their
matching unsigned widths. `UInt40be` emits big-endian five-byte values,
`UInt48be` emits big-endian six-byte values, `UInt56be` emits big-endian
seven-byte values, and `UInt64be` emits big-endian eight-byte values.
Standalone visible `UInt1` through `UInt7` fields
emit one byte with the value in the declared low bits. Values outside the
primitive range return
`Err(EncodeError("codec.encode_value_unrepresentable", field_path, reason))`;
nested schema encode failures keep the nested schema field path. `UInt31be`
and `UInt31le` use the 31-bit maximum even though they occupy four bytes.
Byte-aligned `ReservedBits(width, value)` fields are representation-only: the
helper omits them from the record and emits the declared fixed value in
declaration order. A `ReservedBits(1, 0)` field immediately before a
`UInt31be` field keeps the shared stream-identifier layout and emits the
required zero high bit in the shared four-byte position. A packed
`ReservedBits(width, value)` field followed by the visible `UIntN` primitive
whose width completes the same one-byte, two-byte, three-byte, or four-byte
big-endian storage unit is also representation-only: the helper emits the
high reserved bits from the declared value and the low visible bits from the
encoder input record. A supported prefix group with
`ReservedBits(width, value)` followed by two visible sub-byte or byte-width
`UIntN` fields whose widths complete one byte, a two-byte big-endian storage
unit, a three-byte big-endian storage unit, or a four-byte big-endian storage
unit writes the declared reserved value first, then the two visible values in
declaration order. The two-byte encode form accepts reserved prefix widths
one through fourteen when the visible fields complete the remaining bits, the
three-byte encode form accepts reserved prefix widths seventeen through
twenty-three when the visible fields complete the remaining bits, the
four-byte encode form accepts reserved prefix widths twenty-five through
thirty-one when the visible fields complete the remaining bits, and reports
`codec.encode_value_unrepresentable`
at the out-of-range visible field. A
visible `UIntN` field followed by a
`ReservedBits(width, value)` suffix that completes the same one-byte,
two-byte, three-byte, or four-byte big-endian storage unit is
representation-only in the same way, but emits the visible value in the high
bits and the declared reserved value in the low bits. A visible `UIntN`
field, middle `ReservedBits(width, value)` field, and following visible
`UIntN` field whose widths complete the same storage unit are also
representation-only: the helper writes both visible values around the
declared reserved value in declaration order and reports
`codec.encode_value_unrepresentable` at the out-of-range visible field.
The same middle encode rule includes the narrow two-byte interleaved layout
where a sub-byte visible field and sub-byte middle reserved field are followed
by `UInt8` and a final sub-byte visible field.
The same shared-storage encode rule also covers consecutive non-byte-aligned
`UIntN` and `ReservedBits(width, value)` fields when the group contains at
least one visible field and at least one reserved field, every visible field
is a big-endian sub-byte `UIntN`, and the declared widths complete one byte or
the same two-byte, three-byte, or four-byte big-endian storage unit. The
helper writes visible and reserved values in declaration order, omits reserved
fields from the encoder value record, and reports
`codec.encode_value_unrepresentable` at the out-of-range visible field.
Unsupported non-byte-aligned reserved-bit encode shapes report
`schema.reserved_bits_encode`.
This slice excludes selected mappings that cannot reconstruct all schema-local
encode fields through direct source-field assignments, mapping expressions
that cannot be projected back to schema-local fields, recursive dispatch
payload schemas, dispatch payload schemas outside the generated helper slice,
nested mappings, and derived codec encode execution for unsupported schemas.

The narrow frame decode helper extends the frame-header layout with a bounded
payload view. It first applies the same header validation, then returns the
visible header fields plus `payload: ByteView`. The payload view shares the
input chunk,
starts immediately after the nine-byte frame header, and uses the decoded
length field as its count. If the closed input has fewer payload bytes than
the decoded length, the helper returns `schema.length_out_of_bounds` with the
first missing byte offset, `Http2FrameHeader.payload` field path, expected
payload count, available payload count, and structured byte preview fields.

Standard `StreamInput` values execute as ordinary immutable source ADT values:
`Chunk(bytes)` preserves the supplied `ByteChunk`, including an empty chunk,
and `End` is a separate nullary variant.

The pending-input byte chunk example appends `StreamInput.Chunk` bytes into an
ordinary immutable retained `ByteChunk`, rejects appends that would exceed the
source-owned retained-input `ByteCount` limit, takes a bounded `ByteView` over
the consumed prefix, drops that prefix from the retained chunk, and advances
the separately tracked absolute `ByteOffset`. The example also materializes
the consumed view into an owned `ByteChunk` and reads it after the retained
pending input has advanced. Outgoing protocol action values collect immutable
`ByteChunk` values into a `List<ByteChunk>` in source code; they do not perform
socket writes or introduce a new output storage type.

Executable specification cases may define named binary fixture records inside
their example source or helper files. These test-owned records can carry the
fixture name, decoded `ByteChunk`, optional consumed `ByteCount`, and expected
invalid-fixture error text. The CLI toolchain harness compares their observable
output with complete lowercase hex strings, byte diagnostic metadata, and
stable error text from `case.toml`. A named fixture can decode successfully and
still be intentionally too short for a closed-input `ByteView` read; in that
case, `run --json` reports `codec.incomplete_input` rather than a fixture text
validation failure. A named fixture can also decode successfully and fail a
test-owned codec or protocol field check; in that case, the harness metadata
records the diagnostic id, byte offset, structured field path, and consumed
count where the case has one. This fixture support is limited to executable
specification evidence and does not add a production binary serialization or
fixture API. The same harness support can assert named output `ByteChunk`
lists through complete lowercase hex chunks in `case.toml`, preserving chunk
order and distinguishing empty lists from zero-length chunks.

The source-backed `byte_expect_fixed_u8_be` helper reads one byte from a
`ByteView`, returns `Ok(Int)` when it matches the expected fixed value, and
returns `Err(String)` with `schema.fixed_field_mismatch` byte diagnostic
details when the byte is present but differs. Truncated input remains
`codec.incomplete_input`.

User-defined `fn` declarations are stack-safe for direct self-recursive chains
when every direct self call appears in tail position and the function has no
runtime `ensure` or `invariant` clauses. The final expression of a
function body is tail position. For a tail-position `match`, each arm result
expression is tail position, recursively through nested tail-position
matches. A direct self call in binary or prefix operands, call arguments,
aggregate literals, field access, `?`, `let` initializers, match scrutinees,
or non-final expression statements is not tail position. Calls through
function-typed values are not tail-recursive steps and keep ordinary call
lowering. Eligible tail-recursive steps evaluate the next call arguments
before rebinding parameters for the next logical invocation. Runtime `require`
checks still run at each logical function entry. Non-tail recursion, mutual
recursion, indirect recursion, and functions with runtime return checks,
including runtime `ensure` or `invariant` clauses, keep ordinary call lowering
and do not receive a stack-safety guarantee. The lowering strategy is
backend-owned and does not expose trampoline classes, continuation layout,
syntax, annotations, warnings, or machine-readable eligibility output as
language behavior.

Bounded channel values are backend-owned runtime handles. `channel::bounded`
and `channel::bounded<T>` return a record with `tx` and `rx` fields.
`channel::clone(tx)` returns another sender endpoint for the same channel.
Sending freezes the sent value before crossing the channel boundary. On a
positive-capacity channel, sending waits while the queue is full and then
returns `Ok(())` after the value is queued. Receiving blocks until a queued
value is available or the sender endpoint is closed. It returns `Some(value)`
for a received value and `None` after the channel is closed and drained. A
capacity of zero creates a no-buffer rendezvous channel. It has no queue
storage: sending waits until a receiver is ready, transfers the value directly,
and then returns `Ok(())`. A waiting receive on a zero-capacity channel returns
`Some(value)` when the paired send transfers a value.
Closing the sender endpoint prevents later sends from succeeding and wakes
waiting receivers.
`channel::select(left, right)` observes two receivers with the same item type.
It returns the first ready value as `Some({index, value})`, using `0` for the
left receiver and `1` for the right receiver, and returns `None` only after
both receivers are closed and drained. If both receivers are ready during one
runtime poll, repeated selections rotate the first polled receiver so that
ties alternate between `0` and `1`.
`channel::select_priority(left, right)` has the same receiver and return
behavior, except ties in one runtime poll always choose the left receiver.
`channel::select_many_priority(receivers)` accepts a non-empty
`List<Receiver<T>>`, uses the zero-based index from that list in
`Some({index, value})`, and returns `None` only after all receivers in the list
are closed and drained. If multiple receivers are ready during one runtime
poll, the earliest receiver in the supplied list wins.
`channel::select_many_timeout(receivers, timeout_ms)` has the same receiver
list, return shape, and priority behavior as `channel::select_many_priority`.
It also returns `None` when no receiver has a ready value before the
non-negative millisecond timeout elapses. A negative timeout waits without a
timeout, matching `channel::select_many_priority`.
`channel::select_many_timeout_result(receivers, timeout_ms)` has the same
receiver list, priority behavior, and timeout behavior as
`channel::select_many_timeout`, but reports the wait outcome through the result
boundary used by the other fallible selection helpers.
`channel::select_many_timeout_cancellable(receivers, timeout_ms, token)` uses
the same receiver list, priority behavior, timeout behavior, and result value
shape, and additionally observes the supplied `CancelToken`. It returns
`Err(SelectError)` when that token is already cancelled or becomes cancelled
before a ready receiver wins.
`channel::select_timeout(left, right, timeout_ms)` has the same receiver,
return, and rotating tie-breaking behavior. It also returns `None` when no
value is selected before the non-negative millisecond timeout elapses. A
negative timeout waits without a timeout, matching `channel::select`.
`channel::select_timeout_cancellable(left, right, timeout_ms, token)` has the
same two-receiver order, rotating tie-breaking, timeout behavior, and result
value shape, and additionally observes the supplied `CancelToken`. It returns
`Err(SelectError)` when that token is already cancelled or becomes cancelled
before a ready receiver wins.
`channel::select_result`, `channel::select_priority_result`,
`channel::select_many_timeout_result`,
`channel::select_many_timeout_cancellable`, and
`channel::select_timeout_result`, and
`channel::select_timeout_cancellable` use the same readiness, tie-breaking,
closed-channel, and timeout rules as their non-result counterparts. They
return `Ok(Some(selected))` when a receiver produces a value, `Ok(None)` when
selection closes or times out without a value, and `Err(SelectError)` when
cooperative cancellation interrupts the waiting selection.

Task values are backend-owned runtime handles. `task::spawn` starts a
zero-argument callable on a JVM thread. `task::spawn_with` starts a
one-argument callable on a JVM thread after freezing the ordinary source value
argument at the task boundary. `task::spawn_with2` starts a two-argument
callable on a JVM thread after freezing both ordinary source values at the task
boundary. `task::spawn_with3` starts a three-argument callable on a JVM thread
after freezing all three ordinary source values at the task boundary.
`task::spawn_with4` starts a four-argument callable on a JVM thread after
freezing all four ordinary source values at the task boundary.
`task::spawn_with5` starts a five-argument callable on a JVM thread after
freezing all five ordinary source values at the task boundary.
`task::spawn_with6` starts a six-argument callable on a JVM thread after
freezing all six ordinary source values at the task boundary.
`task::spawn_with7` starts a seven-argument callable on a JVM thread after
freezing all seven ordinary source values at the task boundary.
`task::spawn_with8` starts an eight-argument callable on a JVM thread after
freezing all eight ordinary source values at the task boundary.
`task::spawn_with9` starts a nine-argument callable on a JVM thread after
freezing all nine ordinary source values at the task boundary.
`task::spawn_with10` starts a ten-argument callable on a JVM thread after
freezing all ten ordinary source values at the task boundary.
`task::spawn_with11` starts an eleven-argument callable on a JVM thread after
freezing all eleven ordinary source values at the task boundary.
`task::spawn_with12` starts a twelve-argument callable on a JVM thread after
freezing all twelve ordinary source values at the task boundary.
`task::spawn_with13` starts a thirteen-argument callable on a JVM thread after
freezing all thirteen ordinary source values at the task boundary.
`task::spawn_with14` starts a fourteen-argument callable on a JVM thread after
freezing all fourteen ordinary source values at the task boundary.
`task::spawn_with15` starts a fifteen-argument callable on a JVM thread after
freezing all fifteen ordinary source values at the task boundary.
`task::spawn_with16` starts a sixteen-argument callable on a JVM thread after
freezing all sixteen ordinary source values at the task boundary.
`task::spawn_with17` starts a seventeen-argument callable on a JVM thread
after freezing all seventeen ordinary source values at the task boundary.
`task::spawn_with18` starts an eighteen-argument callable on a JVM thread
after freezing all eighteen ordinary source values at the task boundary.
`task::spawn_with19` starts a nineteen-argument callable on a JVM thread after
freezing all nineteen ordinary source values at the task boundary.
`task::spawn_with20` starts a twenty-argument callable on a JVM thread after
freezing all twenty ordinary source values at the task boundary.
`task::spawn_with21` starts a twenty-one-argument callable on a JVM thread
after freezing all twenty-one ordinary source values at the task boundary.
`task::spawn_with22` starts a twenty-two-argument callable on a JVM thread
after freezing all twenty-two ordinary source values at the task boundary.
`task::spawn_with23` starts a twenty-three-argument callable on a JVM thread
after freezing all twenty-three ordinary source values at the task boundary.
`task::spawn_with24` starts a twenty-four-argument callable on a JVM thread
after freezing all twenty-four ordinary source values at the task boundary.
`task::spawn_with25` starts a twenty-five-argument callable on a JVM thread
after freezing all twenty-five ordinary source values at the task boundary.
`task::spawn_with26` starts a twenty-six-argument callable on a JVM thread
after freezing all twenty-six ordinary source values at the task boundary.
`task::spawn_with27` starts a twenty-seven-argument callable on a JVM thread
after freezing all twenty-seven ordinary source values at the task boundary.
`task::spawn_with28` starts a twenty-eight-argument callable on a JVM thread
after freezing all twenty-eight ordinary source values at the task boundary.
`task::spawn_with29` starts a twenty-nine-argument callable on a JVM thread
after freezing all twenty-nine ordinary source values at the task boundary.
`task::spawn_with30` starts a thirty-argument callable on a JVM thread after
freezing all thirty ordinary source values at the task boundary.
All task spawn helpers freeze the returned value before it crosses back through the
task handle. `task::join` waits for that task and returns `Ok(value)` on ordinary
completion or `Err(JoinError)` on interruption, cancellation, or runtime
failure. `task::cancel` requests cooperative cancellation by interrupting the
task.

File-system intrinsics are backend-owned runtime operations. `fs::read_to_string`
reads UTF-encoded text and returns `Ok(text)` or `Err(FsError)`.
`fs::write_string` writes UTF-encoded text and returns `Ok(())` or
`Err(FsError)`. `fs::exists` returns `Ok(Bool)` for the host existence check or
`Err(FsError)` if the path cannot be interpreted. `fs::read_dir` returns
`Ok(Vec<Path>)` containing backend-owned path values for directory entries or
`Err(FsError)`. These operations use `Result` at the Veln boundary instead of
exposing host exceptions.

Network and time boundary intrinsics are backend-owned runtime operations.
`net::receive_chunk` returns a host-fed immutable `ByteChunk`.
`net::send_chunk` exposes an outgoing immutable `ByteChunk` to the host
runtime and returns `()`. `net::listen` returns a source-visible `NetListener`,
`net::accept` returns a source-visible `NetStream`, `net::accept_or_end`
returns `Some(stream)` for a fixture-accepted stream and `None` for clean end
of the fixture listener, `net::accept_until` returns `Some(stream)` when the
fixture accepts before the supplied `Deadline` and `None` when the fixture
reports deadline expiry before accepting or the supplied `Deadline` has
already expired, `net::read_chunk` reads one immutable `ByteChunk` from that
stream, `net::read_chunk_until` returns `Some(bytes)` when the fixture stream
yields a chunk before the supplied `Deadline` and `None` when the fixture
reports deadline expiry before a chunk is read, the supplied `Deadline` has
already expired, or the fixture stream reaches clean end before a chunk is
read, `net::read_chunk_or_end` returns `Some(bytes)` for a successful stream
read and `None` for clean end of the fixture stream, and `net::write_chunk`
writes one immutable `ByteChunk` to that stream. `net::close_stream` records a
fixture-backed close event for an adapter-owned stream and returns `()`.
`time::timeout_ms` waits for a
non-negative millisecond duration at the runtime boundary and returns `()`.
`time::deadline_after_ms` returns a source-visible `Deadline` for a relative
millisecond duration, and `time::wait_until` waits until that deadline expires.
`time::cancel_token` returns a source-visible `CancelToken`,
`time::cancel` requests cancellation through that handle, `time::is_cancelled`
observes the handle state as `Bool` without waiting or requesting
cancellation, and
`time::wait_until_cancellable` waits until a deadline expires unless the
handle is cancelled first. `time::wait_until_cancellable_outcome` uses the
same deadline and token values and returns `WaitCompleted`,
`WaitDeadlineExpired`, or `WaitCancelled` as a source-visible
`CancellableWaitOutcome`.
Malformed host-fed receive or read bytes, failed outgoing send, write, or
close event recording, and host-fixture-forced listen, accept, read, write,
close, timeout, or deadline expiry, or cancellable-wait cancellation through
the runtime-failure wait stop the entry as runtime failures. Clean listener end
observed through
`net::accept_or_end`, accept deadline expiry observed through
`net::accept_until`, clean stream end observed through `net::read_chunk_or_end`,
read deadline expiry or clean stream end observed through
`net::read_chunk_until`, and value-returning cancellable wait outcomes are
successful source values. Forced accept failure through `net::accept_until`
and forced read failure through `net::read_chunk_until` stay runtime failures.
They do not produce schema, codec, or HTTP/2 peer protocol diagnostics. The
deadline boundary does not add a source timer handle beyond the returned
`Deadline`, cancellation handle beyond `CancelToken`, routing API, or new
effect label.
The stream adapter cancellable routing cases compose those outcome values with
channel-routed `StreamInput` values and ordinary response action values.
The receiver-list cancellable channel-first case selects ordinary
`StreamInput` values with `channel::select_many_timeout` before translating
completed wait, deadline-expired, and cancelled outcomes into response action
values. Completed waits keep the handler-produced actions, deadline expiry
prepends a retry action, and cancellation prepends a cleanup action without
exposing timer handles or transport effects to the handler.

The stream adapter event boundary is source-level in the current executable
specification. Example-owned `StreamEvent` and `ResponseAction` ADTs model
decoded stream work and protocol response intent as immutable values. A plain
handler receives one event and an explicit state record, then returns a list of
response actions and the next state. The same handler can be called directly
by a fixture or after an event crosses a standard channel under the existing
`concurrency` effect. Sending response actions, ending streams, resetting
streams, and declining work are represented as values for the adapter to
interpret; the handler does not call `net::send_chunk`, own sockets, or add
new listen, read, write, routing, or deadline effect labels.

The socket stream adapter routing cases compose that handler boundary with the
fixture-backed socket calls without adding a service interface or new effect
labels. Adapter code owns the `NetListener` and `NetStream`, reads multiple
immutable `ByteChunk` values with `net::read_chunk` or
`net::read_chunk_or_end`, routes ordinary source values through a standard
channel under `concurrency`, calls the plain handler with explicit state, and
then walks the returned action list. Optional accept cases use
`net::accept_or_end` to accept a usable stream as `Some(stream)` or observe a
clean listener end as `None`. Deadline-aware optional accept cases use
`net::accept_until` to accept a usable stream before the deadline or observe
deadline expiry as `None`. Deadline-aware optional read cases use
`net::read_chunk_until` to read bytes before the deadline or observe deadline
expiry or clean stream end as `None`. The clean stream-end case translates
`net::read_chunk_or_end` returning `None` into the standard `StreamInput.End`
value before calling the pure handler. The owned-lifecycle case combines
`net::listen`, `net::accept_or_end`, repeated `net::read_chunk_or_end`, channel
routing, pure handler invocation, and ordered `net::write_chunk` projection in
one adapter path. The deadline-aware lifecycle case combines
`net::accept_until`, repeated `net::read_chunk_until` attempts, channel
routing, pure handler invocation, and ordered write projection in one accepted
stream adapter path, with deadline expiry becoming the ordinary stream
boundary value before handler invocation. Close-lifecycle cases call
`net::close_stream` after ordered writes or cancellation cleanup and record the
close event at the fixture runtime boundary. The same checked boundary also joins
a spawned stream-handler task that passes ordinary event, state, and adapter
context values plus one routing metadata value and two additional ordinary
metadata values directly through `task::spawn_with6` instead of bundling them
into one adapter record first. The seven-argument stream-task case extends the
same boundary with one more ordinary metadata value through
`task::spawn_with7`, and the eight-argument stream-task case extends it with
one additional ordinary metadata value through `task::spawn_with8`. The
nine-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with9`. The ten-argument stream-task case
extends it with one additional ordinary metadata value through
`task::spawn_with10`. The eleven-argument stream-task case extends it with one
additional ordinary metadata value through `task::spawn_with11`. The
twelve-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with12`. The thirteen-argument stream-task
case extends it with one additional ordinary metadata value through
`task::spawn_with13`. The fourteen-argument stream-task case extends it with
one additional ordinary metadata value through `task::spawn_with14`. The
fifteen-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with15`. The sixteen-argument stream-task
case extends it with one additional ordinary metadata value through
`task::spawn_with16`. The seventeen-argument stream-task case extends it with
one additional ordinary metadata value through `task::spawn_with17`. The
eighteen-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with18`. The nineteen-argument stream-task
case extends it with one additional ordinary metadata value through
`task::spawn_with19`. The twenty-argument stream-task case extends it with one
additional ordinary metadata value through `task::spawn_with20`. The
twenty-one-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with21`. The twenty-two-argument
stream-task case extends it with one additional ordinary metadata value through
`task::spawn_with22`. The twenty-three-argument stream-task case extends it
with one additional ordinary metadata value through `task::spawn_with23`. The
twenty-four-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with24`. The twenty-five-argument
stream-task case extends it with one additional ordinary metadata value
through `task::spawn_with25`. The twenty-six-argument stream-task case
extends it with one additional ordinary metadata value through
`task::spawn_with26`. The twenty-seven-argument stream-task case extends it
with one additional ordinary metadata value through `task::spawn_with27`.
The twenty-eight-argument stream-task case extends it with one additional
ordinary metadata value through `task::spawn_with28`.
The twenty-nine-argument stream-task case extends it with one additional
ordinary metadata value through `task::spawn_with29`.
The thirty-argument stream-task case extends it with one additional ordinary
metadata value through `task::spawn_with30`.
`SendBytes` actions are translated into ordered `net::write_chunk` calls by the
adapter. Non-write response intents remain ordinary values for the adapter to
interpret. The handler has no socket handle parameter and does not call `net`
functions. Explicit adapter-owned stream close remains an adapter call. The
checked examples are
`examples/specification/run/socket-stream-adapter-routing/`,
`examples/specification/check/socket-stream-adapter-routing-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn7/`,
`examples/specification/check/socket-stream-adapter-routing-spawn7-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn8/`,
`examples/specification/check/socket-stream-adapter-routing-spawn8-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn9/`,
`examples/specification/check/socket-stream-adapter-routing-spawn9-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn10/`,
`examples/specification/check/socket-stream-adapter-routing-spawn10-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn11/`,
`examples/specification/check/socket-stream-adapter-routing-spawn11-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn12/`,
`examples/specification/check/socket-stream-adapter-routing-spawn12-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn13/`,
`examples/specification/check/socket-stream-adapter-routing-spawn13-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn14/`,
`examples/specification/check/socket-stream-adapter-routing-spawn14-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn15/`,
`examples/specification/check/socket-stream-adapter-routing-spawn15-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn16/`,
`examples/specification/check/socket-stream-adapter-routing-spawn16-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn17/`,
`examples/specification/check/socket-stream-adapter-routing-spawn17-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn18/`,
`examples/specification/check/socket-stream-adapter-routing-spawn18-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn19/`,
`examples/specification/check/socket-stream-adapter-routing-spawn19-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn20/`,
`examples/specification/check/socket-stream-adapter-routing-spawn20-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn21/`,
`examples/specification/check/socket-stream-adapter-routing-spawn21-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn22/`,
`examples/specification/check/socket-stream-adapter-routing-spawn22-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn23/`,
`examples/specification/check/socket-stream-adapter-routing-spawn23-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn24/`,
`examples/specification/check/socket-stream-adapter-routing-spawn24-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn25/`,
`examples/specification/check/socket-stream-adapter-routing-spawn25-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn26/`,
`examples/specification/check/socket-stream-adapter-routing-spawn26-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn27/`,
`examples/specification/check/socket-stream-adapter-routing-spawn27-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn28/`,
`examples/specification/check/socket-stream-adapter-routing-spawn28-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn29/`,
`examples/specification/check/socket-stream-adapter-routing-spawn29-effects/`,
`examples/specification/run/socket-stream-adapter-routing-spawn30/`,
`examples/specification/check/socket-stream-adapter-routing-spawn30-effects/`,
`examples/specification/run/socket-stream-adapter-clean-end/`,
`examples/specification/run/socket-stream-adapter-owned-lifecycle/`,
`examples/specification/check/socket-stream-adapter-owned-lifecycle-effects/`,
`examples/specification/run/socket-stream-adapter-close-lifecycle/`,
`examples/specification/check/socket-stream-close-effects/`,
`examples/specification/run/socket-stream-adapter-deadline-lifecycle/`,
`examples/specification/run/socket-stream-adapter-cancellable-lifecycle/`,
and `examples/specification/run/socket-stream-adapter-cancel-close-lifecycle/`.

The channel-first stream routing cases keep that boundary while routing
ordinary `StreamInput` values through two, three, four, receiver-list
five-route through receiver-list nineteen-route, and receiver-list timeout typed
channel routes
before handler invocation. Adapter
code selects the ready route with existing channel selection and requires
`concurrency`; the receiver-list priority cases use
`channel::select_many_priority` on a non-empty `List<Receiver<StreamInput>>`
and preserve the supplied list order as the priority order. The timeout case
uses `channel::select_many_timeout` to keep that list priority while returning
`None` when no receiver is ready before the timeout. Socket wrappers that read
`NetStream` input and write response bytes require both `net` and
`concurrency`. The plain handler receives stream input plus explicit
per-stream state and remains free of transport effects. The checked examples
are
`examples/specification/run/channel-first-stream-routing/`,
`examples/specification/run/channel-first-stream-routing-three-route/`,
`examples/specification/run/channel-first-stream-routing-four-route/`,
`examples/specification/run/channel-first-stream-routing-five-route/`,
`examples/specification/run/channel-first-stream-routing-six-route/`,
`examples/specification/run/channel-first-stream-routing-seven-route/`,
`examples/specification/run/channel-first-stream-routing-eight-route/`,
`examples/specification/run/channel-first-stream-routing-nine-route/`,
`examples/specification/run/channel-first-stream-routing-ten-route/`,
`examples/specification/run/channel-first-stream-routing-eleven-route/`,
`examples/specification/run/channel-first-stream-routing-twelve-route/`,
`examples/specification/run/channel-first-stream-routing-thirteen-route/`,
`examples/specification/run/channel-first-stream-routing-fourteen-route/`,
`examples/specification/run/channel-first-stream-routing-fifteen-route/`,
`examples/specification/run/channel-first-stream-routing-sixteen-route/`,
`examples/specification/run/channel-first-stream-routing-seventeen-route/`,
`examples/specification/run/channel-first-stream-routing-eighteen-route/`,
`examples/specification/run/channel-first-stream-routing-nineteen-route/`,
`examples/specification/run/channel-select-many-timeout/`,
`examples/specification/run/channel-select-timeout-cancellable/`,
`examples/specification/run/channel-select-many-timeout-cancellable/`,
`examples/specification/run/channel-select-many-timeout-cancellable-forced-cancel/`,
`examples/specification/run/stream-adapter-cancellable-channel-first-routing/`,
`examples/specification/check/channel-first-stream-routing-effects/`,
`examples/specification/check/channel-first-stream-routing-three-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-four-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-five-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-seven-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-eight-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-nine-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-ten-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-eleven-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-twelve-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-thirteen-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-fourteen-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-fifteen-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-sixteen-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-seventeen-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-eighteen-route-effects/`,
and
`examples/specification/check/channel-first-stream-routing-nineteen-route-effects/`,
and
`examples/specification/check/channel-select-many-timeout-effects/`, and
`examples/specification/check/channel-select-timeout-cancellable-effects/`, and
`examples/specification/check/channel-select-many-timeout-cancellable-effects/`, and
`examples/specification/check/stream-adapter-cancellable-channel-first-routing-effects/`.

Current-process intrinsics are also backend-owned runtime operations.
`process::args` returns the selected entry arguments as a frozen vec of
strings. `process::env` returns `Some(value)` for a present environment key and
`None` for an unavailable key. `process::cwd` returns `Ok(Path)` as a
backend-owned path value for the host current working directory or
`Err(ProcessError)` when the runtime cannot produce one. `process::exit`
terminates the selected host process after clamping the integer status into the
implemented backend status range.

This freeze rule is an observable language boundary only through value
immutability and update semantics. The exact JVM representation, copying
strategy, and later structural-sharing choices remain backend details.

Runtime contract failures stop the selected `run` entry or fail the selected
test case. Human output names the failed clause text, function boundary, source
identity, and blame route. `veln run --json` reports one top-level structured
error record. `veln test --json` embeds runtime contract failures in
the failed case with structured runtime contract details. Entries and tests
that return `Err(value)` are reported with structured runtime result details
in `veln run --json` and `veln test --json`. `require` uses caller blame;
`ensure` uses implementation blame. When `?` propagates an error result
out of a function, the function's `ensure` clauses run before that early
return.

The JVM execution path keeps a persistent class cache for generated JVM
classfile artifacts. Before cached classes are executed, the runner validates a
cache manifest against the emitted class paths and classfile contents expected
for the selected program. Missing manifests, incomplete entries, unexpected
files, and class contents that do not match the expected digest are treated as
invalid cache entries and are regenerated instead of executed. Cache hits may
skip artifact preparation, but command results, stdout, stderr, contract traces,
and captured stdio events are defined as if the selected program was emitted
for that invocation.
