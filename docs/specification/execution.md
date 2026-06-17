# Execution Boundary

This page routes implemented execution facts. Use it before opening the full
execution reference.

## Read First

- Checked core and typed IR are produced only after semantic diagnostics have
  no errors.
- Shared command analysis keeps checked-core readiness and selected-entry
  typed-IR readiness before command-specific execution or write policy.
- Reachable executable blockers include holes, missing expressions,
  constructor arity gaps, call arity gaps, and recognized concurrency calls.
- The ordinary JVM execution path emits classfile artifacts directly; Java
  source generation and Java source compilation are not part of the observable
  command boundary.
- The generated JVM class cache validates manifests and classfile contents;
  invalid or incomplete entries are regenerated before execution.
- Standard `List` traversal helpers execute through runtime support that avoids
  growing the host call stack for large helper traversals.
- Standard byte chunk and byte view helpers execute as pure prelude runtime
  operations and return immutable byte values or `Result` failures for invalid
  values, invalid compact hex fixture text, out-of-bounds counts and ranges,
  big-endian and little-endian fixed-width unsigned read truncation, schema
  fixed-field mismatches, bounded view slicing, and conversion overflow.
  Outgoing chunk-list helpers return ordinary immutable `List<ByteChunk>`
  values. Standard `StreamInput`, `DecodeStep<T>`, `DecodeReadiness`,
  `DecodeError`, `EncodeStep<TState>`, and `EncodeError` values execute as
  ordinary immutable ADT values.
- `ByteView` values cross task and channel freeze boundaries with the same
  bounded bytes, logical `ByteOffset`, and `ByteCount` observed by the sender.
  The checked example is
  `examples/specification/run/binary-byteview-freeze-boundary/`. The runtime
  does not expose a source-visible memory layout or zero-copy guarantee.
- Pending-input examples append immutable `StreamInput.Chunk` bytes, enforce a
  source-owned retained `ByteCount` limit, take and drop bounded `ByteView`
  ranges, preserve absolute `ByteOffset` facts separately, materialize
  consumed views as owned `ByteChunk` values that remain readable after
  retained input advances, and collect outgoing immutable `ByteChunk` values
  without socket calls.
- Fixture-backed `net` and `time` calls are host runtime boundaries:
  descriptor chunk receive/send, listener creation, accept, optional
  clean-end listener accept, deadline-aware optional listener accept, stream
  read, optional clean-end stream read, deadline-aware optional stream read,
  stream write, timeout, deadline waits, and cancellable deadline waits
  execute outside the pure protocol core.
  `CancelToken` handles are source-visible time-boundary values used by
  adapter-owned waits. `time::is_cancelled` observes whether such a handle has
  already been cancelled without waiting or requesting cancellation.
  `CancellableWaitOutcome` values let adapter-owned waits observe completion,
  deadline expiry, or cancellation without stopping the entry. Stream adapter
  examples compose those outcomes with channel-routed `StreamInput` values,
  receiver-list channel-first selection, and ordinary response actions in
  fixture output so completed waits, deadline expiry, and cancellation become
  adapter routing decisions.
  Executable fixtures can set `VELN_TIME_CANCELLABLE_OUTCOMES` to
  a comma-separated sequence of `completed`, `deadline-expired`, and
  `cancelled` values for the value-returning wait path.
  Malformed received or read bytes, failed outgoing send or write event
  recording, and forced listen, accept, read, write, timeout, deadline, or
  cancellable-wait cancellation failures through the runtime-failure wait stop
  the entry as runtime failures rather than schema, codec, or peer protocol
  diagnostics. `net::accept_until` turns accept deadline expiry into `None`,
  and `net::read_chunk_until` turns read deadline expiry into `None`, while
  forced host accept or read failure through those paths remains a runtime
  failure.
- Stream adapter event-boundary examples use ordinary source ADT, record, and
  list values for decoded stream events and response actions. A handler
  receives an event plus explicit state and returns action intent values plus
  the next state. Channel routing uses existing `concurrency` calls; response
  actions do not perform socket writes or introduce new effect labels.
- The socket stream adapter routing example composes the existing
  fixture-backed socket calls with the source-level event/action handler
  boundary. Adapter code can accept a listener as `Some(stream)` or clean end
  as `None`, accept before a deadline as `Some(stream)` or deadline expiry as
  `None`, owns the accepted `NetStream` across optional reads until clean
  stream end, translates clean stream end into `StreamInput.End`, routes
  ordinary events through a standard channel under `concurrency`, carries
  explicit handler state across those events, joins a spawned stream-handler
  task over the same event/action boundary, passes ordinary event, state, and
  adapter context values plus one routing metadata value and one additional
  ordinary metadata value into `task::spawn_with5`, and translates ordered
  `SendBytes` actions into `net::write_chunk` calls. Handler code remains free
  of socket handles and `net` calls. The checked examples are
  `examples/specification/run/socket-stream-adapter-routing/`,
  `examples/specification/run/socket-stream-adapter-clean-end/`,
  `examples/specification/run/socket-stream-adapter-owned-lifecycle/`,
  `examples/specification/check/socket-stream-adapter-owned-lifecycle-effects/`,
  and `examples/specification/run/socket-stream-adapter-deadline-lifecycle/`.
  The owned-lifecycle cases cover the listener-to-clean-stream-end ownership
  and effect boundary, and the deadline lifecycle case covers deadline-aware
  accepted-stream ownership in one adapter function.
- The channel-first stream routing examples route ordinary `StreamInput`
  values through two, three, four, receiver-list five-route, receiver-list
  six-route, receiver-list seven-route, receiver-list eight-route,
  receiver-list nine-route, and
  receiver-list timeout typed channel
  routes, select the next ready route with the existing channel selection
  vocabulary, and only then invoke a plain handler with explicit per-stream
  state. The receiver-list priority examples use
  `channel::select_many_priority` on a non-empty
  `List<Receiver<StreamInput>>`; the timeout example uses
  `channel::select_many_timeout` to preserve supplied list order as priority
  order while returning `None` when no receiver is ready before the timeout.
  When multiple receivers are ready, the earliest receiver in the supplied
  list wins. The handler remains an ordinary source function over stream input
  and state; adapter code owns channel routing, and socket wrappers around the
  same boundary own `NetStream` handles and writes. The checked examples are
  `examples/specification/run/channel-first-stream-routing/`,
  `examples/specification/run/channel-first-stream-routing-three-route/`,
  `examples/specification/run/channel-first-stream-routing-four-route/`,
  `examples/specification/run/channel-first-stream-routing-five-route/`,
  `examples/specification/run/channel-first-stream-routing-six-route/`,
  `examples/specification/run/channel-first-stream-routing-seven-route/`,
  `examples/specification/run/channel-first-stream-routing-eight-route/`,
  `examples/specification/run/channel-first-stream-routing-nine-route/`,
  `examples/specification/run/channel-select-many-timeout/`,
  `examples/specification/run/stream-adapter-cancellable-channel-first-routing/`,
  `examples/specification/check/channel-first-stream-routing-effects/`, and
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
  `examples/specification/check/channel-select-many-timeout-effects/`, and
  `examples/specification/check/stream-adapter-cancellable-channel-first-routing-effects/`.
- The generated binary schema helper execution slice decodes the
  `Http2FrameHeaderWire` field sequence from a `ByteView`: `UInt24be`,
  `UInt8`, `UInt8`, `ReservedBits(1, 0)`, and `UInt31be`. The decoded value
  exposes ordinary `Int` fields for `length`, `kind`, `flags`, and
  `stream_id`. The reserved field is consumed and validated but is not
  exposed in the mapped record. Truncated schema fields report
  `schema.truncated_field`; invalid reserved bits report
  `schema.reserved_bits_mismatch`. Both carry byte offset and schema field
  path details. The checked examples are
  `examples/specification/run/binary-schema-frame-header-decode/`,
  `examples/specification/run/binary-schema-frame-header-truncated-json/`,
  `examples/specification/run/binary-schema-frame-header-truncated-human/`,
  `examples/specification/run/binary-schema-frame-header-reserved-json/`, and
  `examples/specification/run/binary-schema-frame-header-reserved-human/`.
- The `SchemaWidthSample` primitive decode slice consumes `UInt16be` followed
  by `UInt32be` from a `ByteView`. Both visible fields decode to ordinary
  `Int` values. Truncated fields use the same `schema.truncated_field` byte
  diagnostic shape as the frame-header slice, including byte offset, field
  path, expected count, available count, readiness, and structured byte
  preview fields.
- Generated binary schema decode helpers also support `UInt16le`, `UInt24le`,
  `UInt31le`, `UInt32le`, and `UInt64le` as little-endian unsigned
  primitives.
  `UInt64be` is accepted as the matching big-endian eight-byte primitive.
  Both eight-byte forms decode to ordinary `Int` fields for values representable
  as source-visible `Int`, preserve structural `map to` runtime mappings, and
  use the same truncation diagnostic shape as the other exact-width primitives.
  The checked examples are
  `examples/specification/run/binary-schema-u64-widths-decode/` and
  `examples/specification/run/binary-schema-u64-widths-truncated-json/`.
- Generated binary schema decode helpers support standalone visible `UInt1`
  through `UInt7` fields. Each field consumes one byte, exposes the declared
  low bits as an ordinary `Int`, advances by one byte, preserves structural
  `map to` runtime mappings, and uses the same `schema.truncated_field`
  diagnostic shape as other exact-width primitives. The checked examples are
  `examples/specification/run/binary-schema-sub-byte-decode/`,
  `examples/specification/run/binary-schema-sub-byte-decode-human/`,
  `examples/specification/run/binary-schema-sub-byte-truncated-json/`, and
  `examples/specification/run/binary-schema-sub-byte-truncated-human/`.
- Generated binary schema decode helpers support opt-in `Flag8`,
  `Flag16be`, `Flag16le`, `Flag32be`, `Flag32le`, `Flag64be`, and
  `Flag64le` fields as visible flag bitsets. They consume the same byte
  width, byte order, and truncation behavior as `UInt8`, `UInt16be`,
  `UInt16le`, `UInt32be`, `UInt32le`, `UInt64be`, and `UInt64le`, but the
  decoded record fields are source-visible `Flag8(bits)`, `Flag16be(bits)`,
  `Flag16le(bits)`, `Flag32be(bits)`, `Flag32le(bits)`, `Flag64be(bits)`,
  and `Flag64le(bits)` values rather than raw `Int` values. Existing `UInt8`,
  `UInt16be`, `UInt16le`, `UInt32be`, `UInt32le`, `UInt64be`, and `UInt64le`
  declarations continue to decode as ordinary `Int` fields. Pure prelude
  helpers inspect or set `Flag8` bit indexes `0` through `7`, `Flag16be` and
  `Flag16le` bit indexes `0` through `15`, `Flag32be` and `Flag32le` bit
  indexes `0` through `31`, and `Flag64be` and `Flag64le` bit indexes `0`
  through `63`, returning `Result` failures for indexes outside each helper's
  range. Raw-bit helpers expose decoded `Flag8`, `Flag16be`, `Flag16le`,
  `Flag32be`, `Flag32le`, `Flag64be`, and `Flag64le` integer bits and
  construct flag values for encode only when the supplied integer fits the
  matching flag width.
  The checked examples are
  `examples/specification/run/binary-schema-flag8-decode/`,
  `examples/specification/run/binary-schema-flag16be-decode/`,
  `examples/specification/run/binary-schema-flag8-bit-helpers/`,
  `examples/specification/run/binary-schema-flag8-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag8-bit-index-json/`,
  `examples/specification/run/binary-schema-flag8-bit-index-human/`,
  `examples/specification/run/binary-schema-flag16be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag16be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag16be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag16be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag16le-decode/`,
  `examples/specification/run/binary-schema-flag16le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag16le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag16le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag16le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag32be-decode/`,
  `examples/specification/run/binary-schema-flag32be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag32be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag32be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag32be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag32le-decode/`,
  `examples/specification/run/binary-schema-flag32le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag32le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag32le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag32le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag64be-decode/`,
  `examples/specification/run/binary-schema-flag64be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag64be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag64be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag64be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag64le-decode/`,
  `examples/specification/run/binary-schema-flag64le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag64le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag64le-bit-index-json/`, and
  `examples/specification/run/binary-schema-flag64le-bit-index-human/`.
- Generated binary schema decode helpers support bounded
  `Repeat(count_field, Payload)` fields when `count_field` is an earlier
  visible exact-width unsigned field decoded as `Int` and `Payload` is
  `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`,
  `UInt31le`, `UInt32be`, `UInt32le`, `UInt64be`, `UInt64le`, an eligible
  nested binary schema payload, or `ByteView(length_field)` when
  `length_field` is another earlier visible exact-width unsigned field decoded
  as `Int`.
  `Repeat(left_count - right_count, Payload)` uses the
  difference of two earlier visible exact-width unsigned `Int` fields as the
  repeat count. A repeated primitive field decodes to `List<Int>`; a repeated
  nested schema field decodes to a list of the nested schema's decoded record
  shape; and a repeated `ByteView(length_field)` field decodes to
  `List<ByteView>` with each element preserving its bounded bytes in element
  order. The helper reads exactly the computed count in declaration order. A
  negative computed count reports `schema.length_out_of_bounds` at the repeat
  field path. Truncation is reported at the first element that cannot be fully
  read with the usual `schema.truncated_field` details and a schema field path
  that appends an `index` segment before nested schema field segments. The
  checked examples are `examples/specification/run/binary-schema-repeat-decode/`,
  `examples/specification/run/binary-schema-repeat-subtract-decode/`,
  `examples/specification/run/binary-schema-repeat-subtract-negative-json/`,
  `examples/specification/run/binary-schema-repeat-truncated-json/`,
  `examples/specification/run/binary-schema-repeat-truncated-human/`,
  `examples/specification/run/binary-schema-repeat-nested-decode/`, and
  `examples/specification/run/binary-schema-repeat-nested-truncated-json/`,
  `examples/specification/run/binary-schema-repeat-byteview-decode/`, and
  `examples/specification/run/binary-schema-repeat-byteview-truncated-json/`.
- Generated binary schema decode helpers support byte-aligned
  `ReservedBits(width, value)` fields up to four bytes wide as
  representation-only fields. The helper consumes the reserved bytes in
  declaration order, validates the declared fixed value, omits the field from
  the decoded value and structural mapping source values, and reports
  `schema.truncated_field` or `schema.reserved_bits_mismatch` at the reserved
  field path when the input is short or the fixed value differs.
- Generated binary schema decode helpers also support packed reserved
  prefixes: `ReservedBits(width, value)` where `width` is one through seven
  may be followed by the visible `UIntN` primitive whose width completes the
  byte, widths nine through fifteen may be followed by the visible `UIntN`
  primitive whose width completes the same two-byte big-endian storage unit,
  and widths seventeen through twenty-three may be followed by the visible
  `UIntN` primitive whose width completes the same three-byte big-endian
  storage unit, and widths twenty-five through thirty-one may be followed by
  the visible `UIntN` primitive whose width completes the same four-byte
  big-endian storage unit. The helper validates the high reserved bits,
  decodes the low
  visible bits as an ordinary `Int`, omits the reserved field from decoded
  records and mapping source values, and advances by the shared storage width
  for the pair. The inverse suffix layout is also supported: a visible
  `UIntN` field followed immediately by `ReservedBits(width, value)` where
  the two widths complete one byte or the same two-byte, three-byte, or
  four-byte big-endian storage unit. That form decodes the visible value from
  the high bits, validates the low reserved bits at the reserved field path,
  omits the reserved field, and
  advances by the shared storage width. The supported middle layout is a
  visible `UIntN` field, a `ReservedBits(width, value)` field, and another
  visible `UIntN` field whose widths together complete one byte or the same
  two-byte, three-byte, or four-byte big-endian storage unit. That form
  decodes the visible fields from their declared high-to-low positions,
  validates the middle reserved field at the reserved field path, omits the
  reserved field, and advances by the shared storage width. A supported
  prefix group may also place `ReservedBits(width, value)` before two visible
  `UIntN` fields when all three widths complete one byte. That form validates
  the high reserved bits, decodes the following visible fields from their
  declared high-to-low positions, omits the reserved field, and advances by
  the shared storage width. The same shared-storage rule also covers
  consecutive non-byte-aligned `UIntN` and `ReservedBits(width, value)`
  fields when the group contains at least one visible field and at least one
  reserved field, every visible field is a big-endian sub-byte `UIntN`, and
  the declared widths complete one byte or the same two-byte, three-byte, or
  four-byte big-endian storage unit. Reserved fields in the group remain
  representation-only, each reserved value is validated at its own field path,
  and visible fields are decoded from their declared high-to-low positions.
- Exact-width generated binary schema decode helpers preserve each field's
  schema-owned external integer maximum while decoding. A structurally present
  field whose decoded value exceeds that maximum reports
  `schema.integer_out_of_range` at the field byte offset with schema field
  path, byte width, accepted range, actual value, and structured byte preview
  fields.
- Exact-width generated binary schema decode helpers treat a field-local
  equality predicate of the form `field == literal` or `literal == field` as a
  visible schema-owned fixed field when the literal fits the field's external
  integer range. The decoded field remains visible in the result when the
  value matches. A mismatch reports `schema.fixed_field_mismatch` at the field
  byte offset with schema field path, expected value, actual value, and
  structured byte preview fields.
- The binary schema field-local validation slice decodes fields in declaration
  order for generated `byte_decode_<schema>` helpers when every field uses an
  implemented exact-width unsigned binary primitive. It checks each supported
  `where` predicate after its owning field is decoded. Predicates may use the
  current field and earlier decoded fields with comparison, boolean, literal,
  arithmetic, prefix `not`, and grouping forms. Later-field references,
  unknown fields, and ordinary source bindings named by a predicate are
  rejected as unsupported schema predicate references. A failed predicate
  reports `schema.validation_failed` at the owning field byte offset with
  schema field path, predicate text, decoded values, and structured byte
  preview fields.
- The schema value validation slice exposes generated `validate_<schema>`
  helpers for eligible binary schema declarations. The helper accepts the
  schema-local decoded record shape, checks field-local `where` predicates in
  declaration order using the same supported predicate language as generated
  binary decode helpers, and returns the supplied record on success. A failed
  predicate reports `schema.validation_failed` with schema field path,
  predicate text, owning supplied field value, and supplied decoded `Int`
  values. Checked examples are
  `examples/specification/run/schema-value-validation/`,
  `examples/specification/run/schema-value-validation-json/`, and
  `examples/specification/run/schema-value-validation-human/`.
- The binary schema schema-level validation slice checks one `validate`
  predicate after all fields have decoded and after field-local validation has
  succeeded, but before structural mapping returns the decoded value.
  Predicates use the same supported expression subset as field-local `where`
  predicates and may reference only decoded `Int` fields from the same schema.
  Primitive, length, dispatch, repeat, reserved-bit, fixed-field, and
  field-local validation failures win before the schema-level predicate is
  evaluated. A failed predicate reports `schema.validation_failed` at the
  byte offset after the decoded schema body with schema path, predicate text,
  decoded values, and structured byte preview fields.
- The narrow binary schema closed dispatch slice decodes
  `Dispatch(tag_field, tag => Payload, ...)` fields after the referenced tag
  field has been decoded by an earlier exact-width field in the same schema.
  It also decodes `Dispatch(tag_field, length_field, tag => Payload, ...)`
  when the length field is an earlier visible `Int` field and the selected
  payload is read from that bounded byte range. Known dispatch cases consume
  either the selected exact-width unsigned payload primitive and expose an
  ordinary `Int` field, or the selected same-module or imported public nested
  binary schema through the generated schema helper path and expose that
  schema's decoded record shape. Nested
  payload decode failures report the outer dispatch field path, nested schema
  field path, and absolute byte offset from the enclosing input, including
  failures from the nested
  helper's fixed-field, reserved-bit, endian, mapping, and primitive decoding
  behavior. Unknown tags in the closed dispatch report
  `schema.dispatch_unknown_tag` at the dispatch field byte offset with schema
  field path, decoded tag field, decoded tag value, expected tags, and
  structured byte preview fields. Same-module recursive closed-dispatch
  payload cases are eligible only in the length-bounded form when selected
  mappings cover every dispatch case and all mappings resolve to one record
  shape, with at least one non-recursive case as the base case. The recursive
  helper path decodes the nested payload from the bounded dispatch range
  before continuing with later fields and preserves the same outer dispatch
  plus nested schema field path on failures. The checked
  examples are
  `examples/specification/run/binary-schema-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-recursive-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
  `examples/specification/run/binary-schema-imported-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-dispatch-nested-failure-json/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-failure-json/`,
  `examples/specification/run/binary-schema-imported-dispatch-nested-failure-json/`,
  `examples/specification/run/binary-schema-recursive-dispatch-failure-json/`,
  `examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
  and
  `examples/specification/run/binary-schema-closed-dispatch-unknown-human/`.
- Closed dispatch cases may decode to mixed primitive and nested record payload
  shapes only when selected `map to Target when tag_field == literal` clauses
  cover the dispatch tag cases with distinct literals and all selected
  mappings resolve to the same target record shape. Each selected branch
  type-checks `payload` as the payload shape chosen by that branch literal;
  other fields keep their schema-local decoded types. The helper still decodes
  the dispatch case from the already decoded tag value before applying the
  selected mapping. Extension dispatch, selectors other than the dispatch tag,
  and uncovered or duplicate case selectors remain outside this mixed-payload
  boundary. The checked examples are
  `examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-decode/`
  and
  `examples/specification/check/binary-schema-mixed-dispatch-selected-mapping-diagnostics/`.
- The narrow binary schema extension dispatch slice decodes
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields
  after both referenced fields have been decoded by earlier exact-width fields
  in the same schema. Known cases consume either the selected exact-width
  unsigned payload primitive or the selected same-module or imported public
  nested binary schema through the generated schema helper path from the
  bounded payload bytes selected by `length_field`, then expose it as
  `SchemaDispatchPayload::Known(value)`.
  Same-module recursive known payload cases are eligible in the
  length-bounded form when selected mappings cover every known case, all
  mappings resolve to one record shape, and at least one known case is
  non-recursive. Recursive known cases decode through the same generated
  schema helper path within the bounded payload range.
  Unknown cases do not report
  `schema.dispatch_unknown_tag`; they expose
  `SchemaDispatchPayload::Unknown(tag, payload)` where `payload` is a bounded
  `ByteView` over exactly the byte count decoded from `length_field`. If the
  closed input cannot provide that bounded payload, the helper reports
  `schema.length_out_of_bounds` at the first missing payload byte. Nested
  payload decode failures report the nested schema field path and absolute byte
  offset from the enclosing input. The checked examples are
  `examples/specification/run/binary-schema-extension-dispatch-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-recursive-extension-dispatch-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-unknown/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-unknown/`,
  and
  `examples/specification/run/binary-schema-extension-dispatch-length-human/`.
  `examples/specification/run/binary-schema-general-helper-roundtrip/`
  combines `Flag8`, bounded repeat, supported reserved bits,
  `ByteView(left_length - right_length)`, and nested extension dispatch in a
  non-HTTP schema and checks successful decode followed by encode.
- When an eligible generated binary schema decode helper has one structural
  `map to Target` clause, or multiple structural mapping clauses selected by
  `when field == literal` or `when field != literal`, and each target resolves
  to the same decoded record shape whose mapped expressions match the target
  field types, the helper returns the selected mapped ordinary record shape
  instead of the schema-local field shape. Mapping selection reads the already
  decoded `Int` selector field after field-local validation succeeds; selector
  clauses must not overlap for any concrete selector value, so at most one
  mapping is selected. Mapping assignment
  expressions may reference decoded schema fields, construct records,
  construct ADT payloads resolved through the ordinary source module rules, or
  call one pure same-module converter function or one imported public pure
  converter function through a written `use` path or alias. They may also
  select a field from an already supported structural mapping expression after
  the source expression is available, when that source expression has a
  record-shaped type with the selected field. An `Int` target field may also
  use `+`, `-`, and `*` over decoded schema-local `Int` fields and nested
  supported mapping arithmetic expressions. A converter argument is either one
  decoded schema-local field or an already implemented structural mapping expression
  made from decoded schema fields, records, ADT constructors, integer
  arithmetic mapping expressions, and nested combinations of those forms. The
  returned value is then assigned to the target field.
  Mapping assignment targets must name target fields, and every target field
  must be assigned once before execution. The implemented mapped decoded field
  types are exact-width unsigned primitive fields, including standalone
  `UInt1` through `UInt7`, as `Int`; `Flag8` fields as `Flag8`;
  `Flag16be` fields as `Flag16be`; `Flag16le` fields as `Flag16le`;
  `Flag32be` fields as `Flag32be`; `Flag32le` fields as `Flag32le`;
  `Flag64be` fields as `Flag64be`; `Flag64le` fields as `Flag64le`;
  length-bounded
  `ByteView(length_field)` and `ByteView(left_length - right_length)` payload
  fields as `ByteView`; closed nested dispatch payload fields as the
  nested schema record shape; closed mixed dispatch payload fields as the
  selected primitive `Int` or nested schema record shape inside the matching
  selector branch; and extension dispatch payload fields as
  `SchemaDispatchPayload<T>`. Mapping expressions cannot call bare imported
  converter names, private imported converter functions, arbitrary ordinary
  functions, read runtime settings, inspect stream state, recover from decode
  failures, or perform effects. The checked examples are
  `examples/specification/run/binary-schema-sub-byte-decode/`,
  `examples/specification/run/binary-schema-flag8-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag16be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag16le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag32be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag32le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag64be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag64le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag8-mapped-constructor-decode/`,
  `examples/specification/run/binary-schema-flag8-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-flag8-imported-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-mapped-record-decode/`,
  `examples/specification/run/binary-schema-mapped-byteview-decode/`,
  `examples/specification/run/binary-schema-mapped-record-expression-decode/`,
  `examples/specification/run/binary-schema-mapped-constructor-expression-decode/`,
  `examples/specification/run/binary-schema-mapping-arithmetic-decode/`,
  `examples/specification/run/binary-schema-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-mapping-selection-decode/`,
  `examples/specification/run/binary-schema-mapping-selection-not-equal-decode/`,
  `examples/specification/run/binary-schema-mapped-field-selection-decode/`,
  `examples/specification/run/binary-schema-mapped-nested-dispatch-decode/`,
  and
  `examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-decode/`.
- Eligible generated binary schema decode-step helpers named
  `byte_decode_step_<schema>` accept a bounded `ByteView` and explicit base
  `ByteOffset`. When the view has at least the schema's exact-width byte
  count, they return `Decoded(value, consumed)` with `consumed` equal to the
  exact schema byte count. When the open view is shorter, they return
  `NeedMore(NeedBytes(count))` with `count` equal to the minimum buffered byte
  count required before retrying and consume no bytes. Closed-input
  `byte_decode_<schema>` truncation diagnostics remain on the existing
  `Result` helper path.
- Eligible generated binary schema encode helpers named
  `byte_encode_<schema>` accept one record whose fields match the schema-local
  visible exact-width unsigned primitive fields as ordinary `Int` values and
  whose `Flag8`, `Flag16be`, `Flag16le`, `Flag32be`, `Flag32le`,
  `Flag64be`, and `Flag64le` fields are source-visible `Flag8(bits)`,
  `Flag16be(bits)`, `Flag16le(bits)`, `Flag32be(bits)`, `Flag32le(bits)`,
  `Flag64be(bits)`, and `Flag64le(bits)` values. For one
  structural `map to Target` clause whose assignments project every visible
  encode field, the helper accepts the mapping target record shape instead
  and projects those target fields back to the schema-local encode record.
  The narrow inverse projection supports direct schema-local field
  references, record expressions whose fields are direct schema-local visible
  field references, field selection from those record expressions when the
  selected field maps directly to one schema-local visible field, and one
  target field assigned from a direct ADT constructor call whose payload
  arguments use those supported projectable field and record-expression forms
  already supported by the generated encode helper. Single-payload
  constructor wrappers remain limited to the existing single-constructor flag
  and exact-width integer cases unless the payload is that record-expression
  slice. A target value whose ADT constructor does not match the constructor
  expected by the mapping returns
  `Err(EncodeError("codec.encode_mapping_mismatch", field_path, reason))`.
  If the expected constructor payload is not the expected record shape, the
  same `codec.encode_mapping_mismatch` id is returned. These mapped encode
  paths write bytes through the schema-local fields. The checked examples are
  `examples/specification/run/binary-schema-mapped-record-expression-encode/`
  and
  `examples/specification/run/binary-schema-mapped-field-selection-encode/`.
  A
  length-bounded `ByteView(length_field)` or
  `ByteView(left_length - right_length)` payload field is a `ByteView` record
  field and emits exactly the bounded bytes from that view after the earlier
  visible length operand fields are written. Decode computes subtraction
  lengths from the earlier decoded field values, rejects negative results as
  `schema.length_out_of_bounds`, and reports the same diagnostic when the
  computed payload length exceeds the remaining bytes. If the supplied view
  count differs from the earlier length field or computed length expression,
  the helper returns
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))` without emitting partial output. Bounded
  repeated primitive fields are `List<Int>` record fields, repeated nested
  schema fields are list fields whose element type is the nested schema's
  decoded record shape, and repeated `ByteView(length_field)` fields are
  `List<ByteView>` record fields. They emit exactly the number of elements
  named by the earlier count field or by the computed difference of two
  earlier count operands. A list length mismatch, a primitive element outside
  the selected primitive range, a repeated byte-view element whose bounded
  byte count differs from the earlier length field, or a nested element
  representation failure returns
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`; repeated byte-view element failures append the element index to
  the repeated field path, and nested element failures prefix the nested schema
  field path with the repeated field and element index. `Flag8` emits one
  byte through the same representation path as `UInt8`, `Flag16be` emits
  two bytes through the same big-endian representation path as `UInt16be`,
  `Flag16le` emits two bytes through the same little-endian representation
  path as `UInt16le`, `Flag32be` emits four bytes through the same
  big-endian representation path as `UInt32be`, `Flag32le` emits four bytes
  through the same little-endian representation path as `UInt32le`,
  `Flag64be` emits eight bytes through the same big-endian representation
  path as `UInt64be`, and `Flag64le` emits eight bytes through the same
  little-endian representation path as `UInt64le`;
  `bits` values outside the selected flag width return
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`. A byte-aligned `ReservedBits(width, value)` field is
  representation-only: it is omitted from the record and the helper emits the
  declared fixed value in declaration order. A `ReservedBits(1, 0)` field
  immediately before a
  `UInt31be` field keeps the shared stream-identifier layout: it is omitted
  from the record and the helper emits the required zero high bit in the
  shared four-byte position. A packed `ReservedBits(width, value)` field
  followed by the visible `UIntN` primitive whose width completes the same
  one-byte, two-byte, three-byte, or four-byte big-endian storage unit is also
  representation-only: the helper emits the high reserved bits from the
  declared value and the low visible bits from the encoder input record. A
  visible `UIntN` field followed by a `ReservedBits(width, value)` suffix
  that completes the same one-byte, two-byte, three-byte, or four-byte
  big-endian storage unit is representation-only in the same way, but emits
  the visible value in the high bits and the declared reserved value in the
  low bits. A visible `UIntN` field, middle `ReservedBits(width, value)`
  field, and following visible `UIntN` field whose widths complete the same
  storage unit are also representation-only: the helper writes both visible
  values around the declared reserved value in declaration order and reports
  `codec.encode_value_unrepresentable` at the out-of-range visible field.
  A supported prefix group with `ReservedBits(width, value)` followed by two
  visible `UIntN` fields whose widths complete one byte writes the declared
  reserved value first, then the two visible values in declaration order, and
  reports `codec.encode_value_unrepresentable` at the out-of-range visible
  field. The same shared-storage encode rule also covers consecutive
  non-byte-aligned `UIntN` and `ReservedBits(width, value)` fields when the
  group contains at least one visible field and at least one reserved field,
  every visible field is a big-endian sub-byte `UIntN`, and the declared
  widths complete one byte or the same two-byte, three-byte, or four-byte
  big-endian storage unit. The helper writes visible and reserved values in
  declaration order, omits reserved fields from the encoder value record, and
  reports `codec.encode_value_unrepresentable` at the out-of-range visible
  field.
  Closed `Dispatch(tag_field, tag => Payload, ...)` fields are eligible when
  `tag_field` names an earlier visible exact-width unsigned field and every
  case payload is an implemented exact-width unsigned primitive payload or an
  eligible nested binary schema payload named as an earlier same-module binary
  schema or a public imported binary schema through a written `use` path. The
  record contains the visible tag
  field and one payload field; for nested payload schemas the payload field
  uses the nested schema decoded record shape. The helper chooses the case
  from the encoded tag value, writes selected nested payload schemas through
  the generated schema helper path in declaration order, and returns
  `Err(EncodeError("codec.dispatch_unknown_tag",
  field_path, reason))` when the tag value has no case.
  Extension-tolerant
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields are
  eligible for the same exact-width unsigned primitive or eligible nested
  binary schema payload cases when both the tag and length fields are earlier
  visible exact-width unsigned fields. The payload record field is
  `SchemaDispatchPayload<T>`, where `T`
  is the selected primitive `Int` or nested schema decoded record shape.
  `Known(value)` writes the payload selected by the visible tag field.
  `Unknown(tag, payload)` writes the bounded raw bytes from the `ByteView`
  only when the visible tag value is not a known case and matches the unknown
  payload tag.
  Same-module recursive known payload cases use the same selected-mapping
  eligibility as recursive closed dispatch; the generated encode helper
  projects the selected known value to the recursive payload, writes it through
  the same schema helper path, and validates the resulting byte count against
  the explicit length field.
  The supplied length field remains explicit: the helper rejects values whose
  encoded payload byte count differs from the earlier length field with
  `Err(EncodeError("codec.dispatch_length_mismatch", field_path, reason))`.
  Visible tag and payload variant disagreements return
  `Err(EncodeError("codec.dispatch_mismatch", field_path, reason))`.
  The helper writes fields in declaration order into one immutable
  `ByteChunk`, using each primitive's declared byte order, and returns
  `Result<ByteChunk, EncodeError>`. `UInt16le`, `UInt24le`, `UInt31le`,
  `UInt32le`, and `UInt64le` emit little-endian bytes and use the same
  representability boundaries as their matching unsigned widths. `UInt64be`
  emits big-endian eight-byte values. Standalone visible `UInt1` through
  `UInt7` fields emit one byte with the value in the declared low bits. Values
  outside the primitive range return
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`; nested schema encode failures keep the nested schema field path.
  `UInt31be` and `UInt31le` use the 31-bit maximum even though they occupy four
  bytes.
  After the helper has projected the input value to schema-local visible
  fields and checked primitive, fixed-field, length, repeat, and dispatch
  representability, it evaluates supported field-local `where` predicates in
  declaration order over the current visible `Int` field and earlier visible
  `Int` fields. Primitive representation failures and other encode
  representability failures win before field-local predicate failures. A
  failed encode predicate returns
  `Err(EncodeError("schema.validation_failed", field_path, reason))` and
  command value diagnostics preserve the schema field path, predicate text,
  owning field value, and available schema-local `Int` values.
  When a `veln run` entry returns these generated `EncodeError` values
  directly, command diagnostics preserve the source-visible
  `EncodeError(id, field_path, reason)` shape and attach
  `details.value_diagnostic` for
  `codec.encode_value_unrepresentable`, `codec.dispatch_unknown_tag`,
  `codec.dispatch_length_mismatch`, `codec.dispatch_mismatch`, and
  encode-time `schema.validation_failed`. Human
  output keeps the primary message focused on the failed encode fact and
  reports field path, predicate or reason details, and rendered result value
  as related notes.
  When a `veln run` entry returns
  `EncodeStep::Invalid(EncodeError(id, field_path, reason))`, the command
  reports the contained `EncodeError` through the same command-facing value
  diagnostic projection. `Encoded` and `Partial` remain ordinary successful
  source-visible values.
  Unsupported non-byte-aligned reserved-bit encode shapes report
  `schema.reserved_bits_encode`.
  Multiple selected mapping clauses selected by `when field == literal` or
  `when field != literal` are eligible when all clauses resolve to the same
  target record shape and every schema-local encode field, including the
  selector field, projects back from the selected target record through direct
  source-field assignments. The helper selects the mapping whose projected
  selector value satisfies the clause, then uses the same generated encode
  diagnostic shape for selector and projected-field representation failures.
  Same-module recursive
  closed-dispatch and extension-dispatch payload cases are also eligible in
  the length-bounded form when selected mappings cover every dispatch case,
  all mappings resolve to one record shape, and at least one case is
  non-recursive. The generated encode helper writes the selected recursive
  payload through the same schema helper path and checks the encoded payload
  byte count against the earlier length field. This slice excludes selected
  mappings that cannot reconstruct
  all schema-local encode fields, mapping expressions that cannot be projected
  back to schema-local fields, recursive dispatch payload schemas outside that
  selected same-module length-bounded dispatch slice, dispatch payload
  schemas outside the generated helper slice, nested mappings, and derived
  codec encode execution for unsupported schemas.
  The checked examples are
  `examples/specification/run/binary-schema-u64-widths-encode/`,
  `examples/specification/run/binary-schema-u64-widths-encode-out-of-range/`,
  `examples/specification/run/binary-schema-sub-byte-encode/`,
  `examples/specification/run/binary-schema-sub-byte-encode-human/`,
  `examples/specification/run/binary-schema-sub-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-sub-byte-encode-out-of-range-human/`,
  `examples/specification/run/binary-schema-primitive-encode/`,
  `examples/specification/run/binary-schema-flag8-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag16be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag16le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag32be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag32le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag64be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag64le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-mapped-record-encode/`,
  `examples/specification/run/binary-schema-primitive-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag8-encode/`,
  `examples/specification/run/binary-schema-flag16be-encode/`,
  `examples/specification/run/binary-schema-flag16le-encode/`,
  `examples/specification/run/binary-schema-flag32be-encode/`,
  `examples/specification/run/binary-schema-flag32le-encode/`,
  `examples/specification/run/binary-schema-flag64be-encode/`,
  `examples/specification/run/binary-schema-flag64le-encode/`,
  `examples/specification/run/binary-schema-flag8-bit-helpers/`,
  `examples/specification/run/binary-schema-flag8-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag16be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag16le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag32be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag32le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag64be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag64le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag8-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-flag8-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-int-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-int-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-multi-payload-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-multi-payload-mapped-constructor-encode-mismatch/`,
  `examples/specification/run/binary-schema-mapped-constructor-field-selection-encode/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-mismatch/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-mismatch-json/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-byteview-encode/`,
  `examples/specification/run/binary-schema-byteview-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-byteview-subtract-decode/`,
  `examples/specification/run/binary-schema-byteview-subtract-negative-json/`,
  `examples/specification/run/binary-schema-byteview-subtract-truncated-json/`,
  `examples/specification/run/binary-schema-byteview-subtract-encode/`,
  `examples/specification/run/binary-schema-byteview-subtract-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-repeat-encode/`,
  `examples/specification/run/binary-schema-repeat-subtract-encode/`,
  `examples/specification/run/binary-schema-repeat-encode-out-of-range/`,
  `examples/specification/run/binary-schema-repeat-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-subtract-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-nested-encode/`,
  `examples/specification/run/binary-schema-repeat-nested-encode-failure/`,
  `examples/specification/run/binary-schema-repeat-byteview-encode/`,
  `examples/specification/run/binary-schema-repeat-byteview-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-reserved-bit-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-four-byte-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-four-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-packed-reserved-three-byte-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-suffix-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-suffix-encode-out-of-range/`,
  `examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-encode-out-of-range/`,
  `examples/specification/run/binary-schema-packed-reserved-two-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-middle-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-group-decode-encode/`,
  `examples/specification/run/binary-schema-split-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-middle-reserved-json/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-recursive-closed-dispatch-encode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`,
  `examples/specification/run/binary-schema-imported-closed-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode-unknown-tag/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode-out-of-range/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode-unknown/`,
  `examples/specification/run/binary-schema-recursive-extension-dispatch-encode/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-mismatch/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-tag-mismatch/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-out-of-range/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-dispatch-nested-encode-failure/`,
  `examples/specification/run/binary-schema-imported-dispatch-nested-encode-failure/`,
  `examples/specification/run/binary-schema-encode-value-diagnostic-json/`,
  `examples/specification/run/binary-schema-encode-value-diagnostic-human/`,
  `examples/specification/run/binary-schema-encode-validation-json/`,
  `examples/specification/run/binary-schema-mapped-encode-validation-human/`,
  `examples/specification/run/binary-schema-dispatch-unknown-tag-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-unknown-tag-encode-diagnostic-human/`,
  `examples/specification/run/binary-schema-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-length-encode-diagnostic-human/`,
  `examples/specification/run/binary-schema-recursive-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-recursive-extension-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-mismatch-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-mismatch-encode-diagnostic-human/`,
  `examples/specification/run/binary-schema-general-helper-roundtrip/`,
  and
  `examples/specification/check/schema-reserved-bit-encode-diagnostics/`.
- A codec declaration with a valid `derive encode` clause for the same
  eligible generated binary schema encode helper slice exposes the codec item
  name as the executable encode boundary for ordinary source calls, including
  repeat-backed schemas, the implemented direct structural mapping and
  selected structural mapping slices, and eligible nested dispatch payload
  schemas already accepted by `byte_encode_<schema>`.
  The call accepts the generated helper's value record or mapped target
  record, invokes the schema encode helper, returns `EncodeStep<()>`, projects
  helper `Ok(ByteChunk)` output to `Encoded(List<ByteChunk>)` with one chunk,
  and projects helper `Err(EncodeError)` output to `Invalid(EncodeError)`.
  The checked examples are
  `examples/specification/run/derived-codec-encode-boundary/`,
  `examples/specification/run/derived-codec-budgeted-encode-boundary/`,
  `examples/specification/run/derived-codec-mapped-encode-boundary/`,
  `examples/specification/run/derived-codec-selected-mapping-encode-boundary/`,
  `examples/specification/run/derived-codec-record-payload-mapped-encode-boundary/`,
  `examples/specification/run/derived-codec-byteview-encode-boundary/`,
  `examples/specification/run/derived-codec-repeat-encode-boundary/`,
  `examples/specification/run/derived-codec-repeat-byteview-encode-boundary/`,
  `examples/specification/run/derived-codec-nested-dispatch-encode-boundary/`,
  `examples/specification/run/derived-codec-imported-nested-dispatch-encode-boundary/`,
  and
  `examples/specification/run/binary-schema-general-helper-roundtrip/`.
  The general-helper roundtrip case covers the combined non-HTTP schema shape
  and checks both successful `Ok(ByteChunk)` projection and helper
  `Err(EncodeError)` projection.
  The budgeted boundary case calls the same derived codec with the value
  record plus an explicit `ByteCount` output budget. If the generated
  `ByteChunk` fits in the budget, the call returns
  `Encoded(List<ByteChunk>)`; if the chunk exceeds the budget, the call
  returns `Partial(List<ByteChunk>, ByteCount, state)` with the emitted prefix,
  produced byte count, and a state record that carries the original value
  fields plus `encoded_offset: ByteCount`. Passing that state record back to
  the same codec with a later budget resumes at the committed offset. Helper
  `Err(EncodeError)` output still projects to `Invalid(EncodeError)` before
  any output chunk is exposed.
  A `derive encode` clause is rejected with
  `codec.derive_helper_unsupported` when the referenced schema cannot expose
  the required generated encode helper, including mapping expression shapes
  that cannot be projected back to the schema-local encode record.
  `examples/specification/check/derived-codec-mapping-boundary-diagnostics/`
  and
  `examples/specification/check/derived-codec-helper-eligibility-diagnostics/`
  pin those checker boundaries.
- A codec declaration with a valid `derive decode` clause for the same
  eligible generated binary schema decode-step slice exposes the codec item
  name as the executable decode boundary for ordinary source calls, including
  supported middle reserved-bit layouts, repeat-backed schemas, same-module or
  public imported nested dispatch payload schemas, and multiple decoded-field
  selected schema mappings already accepted by `byte_decode_step_<schema>`.
  The call accepts a bounded
  `ByteView` and explicit base `ByteOffset` and returns the same
  `DecodeStep<T>` value as
  `byte_decode_step_<schema>`, including mapped record values,
  `NeedMore(NeedBytes(count))`, and `Invalid` without consumed bytes. The
  checked examples are
  `examples/specification/run/derived-codec-decode-boundary/`,
  `examples/specification/run/derived-codec-middle-reserved-decode-boundary/`,
  `examples/specification/run/derived-codec-repeat-decode-boundary/`,
  `examples/specification/run/derived-codec-nested-dispatch-decode-boundary/`,
  `examples/specification/run/derived-codec-imported-nested-dispatch-decode-boundary/`,
  and
  `examples/specification/run/binary-schema-general-helper-roundtrip/`.
  The general-helper roundtrip case covers the combined non-HTTP schema shape
  and checks successful `Decoded`, short-input `NeedMore`, and helper-failure
  `Invalid` outcomes through the codec item.
  `examples/specification/run/codec-needmore-parser-state/` covers
  caller-owned parser state around the codec boundary: after `Decoded`, the
  caller drops exactly the consumed prefix and advances the explicit base
  offset by the consumed count; after `NeedMore`, the caller keeps the same
  pending bytes and base offset.
  `examples/specification/run/codec-selected-mapping-decode-boundary/`
  covers the selected mapping boundary shared with hand-written decode
  codecs. For the implemented structural mapping slice, `T` is the mapping
  target record shape when each assignment source has the same implemented
  decoded field type as the target field and all selected mappings resolve to
  that same record shape.
  A `derive decode` clause is rejected with
  `codec.derive_helper_unsupported` when the referenced schema cannot expose
  the required generated decode-step helper. The helper eligibility diagnostics
  case listed above pins that checker boundary.
- A codec declaration with a valid hand-written `decode with function_name`
  clause exposes the codec item name as the executable decode boundary for
  ordinary source calls. The call accepts a bounded `ByteView` and explicit
  base `ByteOffset` and invokes the referenced same-module function.
  `NeedMore(readiness)` and `Invalid(error)` return unchanged.
  `Decoded(value, consumed)` returns unchanged when `consumed` is within the
  supplied view length; when `consumed` is outside the supplied view, the codec
  boundary returns `Invalid(DecodeError("codec.consumed_count_invalid",
  base_offset, codec_name))`. When the referenced schema uses multiple
  decoded-field selected mappings that resolve to one implemented target
  record shape, the referenced function must return `DecodeStep<T>` for that
  selected mapping record shape.
- For `veln run` entries, a returned
  `DecodeStep::Invalid(DecodeError(id, byte_offset, field_path))` is
  projected to a focused human runtime diagnostic and
  `details.byte_diagnostic` JSON using the contained diagnostic id, byte
  offset, and field path. A returned `DecodeStep::NeedMore(readiness)` is
  projected at the closed-input reporting boundary as
  `codec.incomplete_input`, with readiness and requested byte count details
  from the source-visible `DecodeReadiness` value. `Decoded` remains an
  ordinary successful entry value. The checked examples are
  `examples/specification/run/codec-decode-invalid-step-human/`,
  `examples/specification/run/codec-decode-invalid-step-json/`,
  `examples/specification/run/codec-decode-need-more-human/`, and
  `examples/specification/run/codec-decode-need-more-json/`.
- A codec declaration with a valid hand-written `encode with function_name`
  clause exposes the codec item name as the executable encode boundary for
  ordinary source calls. The call invokes the referenced same-module function
  with that function's parameters and returns its `EncodeStep<TState>` value
  unchanged, including `Encoded`, `Partial`, and `Invalid` results. A checked
  budgeted encode example observes `Partial` with its emitted chunk list,
  produced byte count, and resumed encoder state as ordinary source-visible
  values, then uses the returned state to complete a later encode call. For
  `veln run` entries, a returned `Invalid(EncodeError(...))` is projected to
  the same focused human and `details.value_diagnostic` JSON diagnostics used
  for command-facing `EncodeError` result values. The checked examples are
  `examples/specification/run/codec-encode-invalid-step-human/` and
  `examples/specification/run/codec-encode-invalid-step-json/`. For the
  implemented structural `map to Target` schema slice, the first encoder
  parameter remains the mapped target record shape.
- Same-module private decode codecs are callable only in their declaring
  module; same-module private encode codecs follow the same rule. Imported
  calls require a written qualified module path to a `pub codec`.
- The frame decode helper reuses the frame-header validation and adds a
  bounded `payload: ByteView` over the same bytes. The payload starts after
  the nine-byte frame header and uses the decoded `length` as its count. If
  the closed input cannot provide that payload range, the helper returns
  `schema.length_out_of_bounds` with byte offset, schema field path, expected
  payload count, available payload count, and structured byte preview fields.
- Executable specification cases may keep named binary fixture records in the
  example tree; the harness checks complete lowercase hex output without
  promoting a production fixture API. Named fixture records can also represent
  valid decoded bytes that are intentionally too short for a closed-input
  `ByteView` read; those cases keep fixture-owned truncation facts in
  metadata while `run --json` reports `codec.incomplete_input`. Other named
  fixture records can represent valid decoded bytes that fail a test-owned
  codec or protocol field check; their metadata records the diagnostic id,
  byte offset, structured field path, and consumed count where the case has
  one. A fixture may also write `schema = "Name"` or
  `schema = "module::Name"` to validate that the fixture metadata names a
  schema from the command source set. Bare names resolve in the fixture's
  source module. Qualified names require a written `use` path and a public
  schema or public schema alias. When `field_path` is present, its first
  segment must name the resolved schema. Invalid fixture schema references are
  manifest validation errors for executable specification cases, not runtime
  `veln run` output. The accepted fixture metadata reference cases cover
  same-module schemas, imported public schemas, and imported public schema
  aliases.
- Executable specification cases may also assert named output `ByteChunk`
  lists through complete lowercase hex in `case.toml`. The harness checks
  stable consecutive program-output lines for the list count, chunk order,
  exact hex strings, decoded byte counts, empty lists, and zero-length chunks.
- Executable specification cases that intentionally reject fixture schema
  metadata may write `[manifest_error]` with `contains = [...]`; the harness
  validates those substrings against the manifest validation failure and does
  not run the command.
- The first ordinary-source HTTP/2 sans-I/O protocol-core example models
  chunk arrival and end-of-stream events as ADTs. Its pure decode state keeps
  undecoded suffix bytes, the next absolute byte offset, client connection
  preface state, continuation state with accumulated opaque header-block
  bytes, the active local receive-limit entry, peer-advertised SETTINGS state,
  outstanding local SETTINGS state, and graceful shutdown state. It validates the
  client connection preface before frame-header decode and represents partial
  or mismatched prefaces, closed-input truncation, continuation ordering
  failures for different frame kinds and stream ids, closed input while a
  header block remains pending, completed HEADERS and multi-frame
  CONTINUATION header-block output, incoming frame payloads that exceed the
  active receive maximum frame size, received `SETTINGS_ENABLE_PUSH`,
  `SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
  `SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_HEADER_TABLE_SIZE`, and
  `SETTINGS_MAX_HEADER_LIST_SIZE` peer-advertised state with item byte
  offsets, unknown SETTINGS identifiers that leave peer-advertised state
  unchanged, multi-item SETTINGS frames where unknown identifiers are ignored
  while known items are applied or diagnosed at their own item byte offset,
  received `SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_FRAME_SIZE`, and
  `SETTINGS_INITIAL_WINDOW_SIZE` values outside their accepted SETTINGS
  ranges, HPACK fixture-codec calls at the completed HEADERS or CONTINUATION
  header-block boundary, local header-list receive-limit checks after fixture
  decoding, zero-length SETTINGS ACK frames that clear outstanding local
  SETTINGS state,
  zero-length SETTINGS ACK frames with no outstanding local SETTINGS state,
  wrong-length SETTINGS ACK payloads with bounded inspected-payload previews,
  stream id domain failures including HEADERS and CONTINUATION on the
  connection stream, invalid stream-state frame kinds, wrong-length
  PING, PRIORITY, GOAWAY, and `RST_STREAM` payloads with bounded
  inspected-payload previews, accepted PING ACK distinction,
  accepted PRIORITY dependency stream id, exclusive flag, and weight facts
  recorded on the tracked open stream, replacement of those tracked priority
  facts by a later accepted PRIORITY frame for the same stream, PRIORITY
  stream-state failures for idle, closed-by-peer, reset, and mismatched
  streams, PRIORITY self-dependency failures, peer-sent `PUSH_PROMISE`
  rejection,
  accepted GOAWAY last-stream-id and error-code, GOAWAY last-stream-id
  enforcement for later peer-created HEADERS streams and local outbound
  HEADERS send-intents above a received boundary, and accepted
  `RST_STREAM` error-code facts as typed protocol values. In the server-side
  fixture core, SETTINGS,
  PING, and GOAWAY require stream id zero; HEADERS, DATA, PRIORITY, `RST_STREAM`,
  `PUSH_PROMISE`, CONTINUATION, and stream-level `WINDOW_UPDATE` require a nonzero
  client-initiated stream id. The receive flow-control state opens an idle
  peer-created stream on an admitted HEADERS frame, counts the tracked open
  peer-created stream for the active concurrent-stream receive limit, consumes
  DATA payload length from connection and stream windows, accepts PADDED DATA
  by consuming the pad-length byte and padding as receive-window credit while
  exposing only application data bytes as DATA content, moves the stream to
  a closed-by-peer state when accepted inbound DATA carries `END_STREAM`, moves
  completed inbound HEADERS or CONTINUATION header blocks to the same
  closed-by-peer state when the accepted HEADERS sequence carries
  `END_STREAM`, accepts
  connection-level and open-stream `WINDOW_UPDATE` increments, applies
  received `SETTINGS_INITIAL_WINDOW_SIZE` deltas to the tracked open-stream
  receive-window credit, and keeps wrong-length, idle-stream, zero,
  closed-by-peer stream, reset-stream, concurrent-stream-limit,
  header-list-size, invalid DATA padding, negative-credit DATA, and overflow
  cases as typed protocol failures.
  Closed-by-peer streams reject later DATA and stream-level `WINDOW_UPDATE`
  through the same stream-state protocol failure shape used by other
  non-open stream states. A received `RST_STREAM` on the open stream
  clears that stream and stores the reset error code so later DATA or
  stream-level `WINDOW_UPDATE` cannot treat the stream as open. A received
  `PUSH_PROMISE` is a known HTTP/2 frame kind and is rejected by the
  server-side receive core through `http2.protocol.invalid_frame_kind` instead
  of falling back to unknown extension-frame preservation. After the client
  preface gate, structurally
  complete unknown extension frame types decode to ordinary `UnknownFrame`
  values that preserve frame type, flags, stream id, payload length, and each
  bounded payload byte, with the preserved payload also checked as complete
  lowercase hex output; an active continuation sequence still rejects an
  unknown frame through the existing continuation protocol-state failure before
  projecting stable diagnostic ids and related context into fixture output,
  human runtime diagnostics, and
  `run --json`
  `protocol_diagnostic` details.
- The HTTP/2 protocol-core HPACK fixture boundary models HPACK as an imported
  ordinary source module, not as schema syntax. The fixture module accepts a
  small deterministic set of header-block byte fixtures, including the HPACK
  static indexed `0x82` `:method: GET`, `0x83` `:method: POST`, `0x84`
  `:path: /`, `0x85` `:path: /index.html`, `0x86` `:scheme: http`, and
  `0x87` `:scheme: https`, plus `0x88` `:status: 200`, `0x89`
  `:status: 204`, `0x8a` `:status: 206`, `0x8b` `:status: 304`, `0x8c`
  `:status: 400`, `0x8d` `:status: 404`, `0x8e` `:status: 500`,
  `0x8f` `accept-charset:`, `0x90` `accept-encoding: gzip, deflate`, and
  `0x91` `accept-language:`, plus `0x92` `accept-ranges:`, `0x93`
  `accept:`, `0x94` `access-control-allow-origin:`, `0x95` `age:`,
  `0x96` `allow:`, `0x97` `authorization:`, `0x98` `cache-control:`,
  plus `0x99` `content-disposition:`, `0x9a`
  `content-encoding:`, `0x9b` `content-language:`, `0x9c`
  `content-length:`, `0x9d` `content-location:`, `0x9e`
  `content-range:`, `0x9f` `content-type:`, `0xa0` `cookie:`, `0xa1`
  `date:`, and `0xa2` `etag:` bytes, plus the no-Huffman
  literal-without-indexing fixture `04 07 2f 74 61 72 67 65 74` for
  `:path: /target`,
  returns
  ordinary header-list data plus the next immutable fixture state, and projects
  unsupported fixture input, including malformed literal-without-indexing
  variants, through
  `hpack.fixture.unsupported_header_block`. That diagnostic path is distinct
  from `schema.*`, `http2.protocol.*`, and `http2.peer_limit.*` ids; the
  HTTP/2 core still owns the local
  `http2.peer_limit.header_list_size_exceeded` receive-limit boundary after
  fixture decoding.
- The same example keeps outbound DATA send-intent flow control separate from
  inbound receive limits. Received `SETTINGS_MAX_FRAME_SIZE` constrains DATA
  payloads this endpoint sends, received `SETTINGS_INITIAL_WINDOW_SIZE`
  supplies the outbound stream credit for the peer-owned stream window, and
  valid DATA intents first check the full payload against available outbound
  connection and stream credit. Payloads larger than the peer-advertised
  maximum frame size are emitted in one immutable output chunk containing
  multiple DATA frames, each no larger than that maximum, with `END_STREAM`
  only on the final DATA frame when requested. PADDED DATA send-intents encode
  the PADDED flag, one pad-length byte per emitted frame, application bytes,
  and requested zero padding bytes; frame-size splitting and outbound credit
  checks count the full encoded DATA payload for each frame, including the
  pad-length byte and padding. Padding that cannot fit in the selected frame
  payload is rejected before output bytes or credit changes. Accepted DATA
  consumes outbound connection and stream credit by the full encoded DATA
  payload length after all split frames encode. Over-window DATA intents are
  rejected before output bytes or credit changes. Accepted outbound DATA with
  `END_STREAM` records local closed-stream state; later outbound DATA,
  outbound HEADERS, and
  stream-level outbound `WINDOW_UPDATE` for that stream follow the existing
  closed stream-state rejection boundary.
  Outbound `WINDOW_UPDATE` send-intents accept connection-level and
  currently open stream-level receive-credit increments, emit one immutable
  frame with a four-byte increment payload, reject zero, negative,
  out-of-range, current-window overflow, stream id zero, idle-stream,
  closed-stream, reset-stream, and mismatched-stream intents before output
  bytes, and leave generated helper representation failures for the frame
  header or increment payload as a
  `codec.encode_value_unrepresentable` encode error rather than an HTTP/2
  diagnostic.
- The same HTTP/2 protocol-core example also covers the narrow outbound frame
  header encode slice. Ordinary source builds a record-shaped frame
  description with `length`, `kind`, `flags`, and `stream_id`, passes it to an
  eligible generated binary schema encode helper, and observes one nine-byte
  immutable `ByteChunk` with the `ReservedBits(1, 0)` plus `UInt31be` stream
  identifier layout. The checked case pins complete lowercase hex output for a
  SETTINGS header on the connection stream, a DATA header on a nonzero stream,
  and the maximum valid `UInt31be` stream id. A stream id outside the generated
  helper's range returns
  `EncodeError("codec.encode_value_unrepresentable", field_path, reason)`
  without adding an HTTP/2-specific diagnostic.
- The same HTTP/2 protocol-core example also covers the narrow outbound
  SETTINGS ACK send-intent. After a valid non-ACK SETTINGS receive, ordinary
  source reuses the frame-header encode path to construct exactly one
  immutable nine-byte output chunk with length `0`, kind `4`, flags `1`, and
  stream id `0`. That send intent only constructs the output chunk; it does
  not update peer-advertised SETTINGS state or local receive-limit state.
- The same example also covers the narrow local SETTINGS send-intent and ACK
  tracking slice. Ordinary source constructs exactly one SETTINGS item for
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_INITIAL_WINDOW_SIZE`,
  `SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
  `SETTINGS_MAX_FRAME_SIZE`, or `SETTINGS_MAX_HEADER_LIST_SIZE`, emits the
  frame-header-plus-item output
  chunk with the selected item identifier and four-byte unsigned value, and
  records one outstanding local SETTINGS batch in connection state with that
  identifier and item count. Local `SETTINGS_ENABLE_PUSH` accepts values `0`
  and `1`; other values are rejected before bytes are emitted with the
  SETTINGS value range failure shape. A valid received SETTINGS ACK clears
  that outstanding state. A valid received SETTINGS ACK when no local SETTINGS
  batch is outstanding fails as
  `http2.protocol.unexpected_settings_ack` with active state and rule
  provenance in related context.
- The same HTTP/2 protocol-core example also covers the narrow outbound PING
  ACK send-intent. After a valid inbound non-ACK PING frame, ordinary source
  reuses the frame-header encode path to construct exactly one immutable
  output chunk with length `8`, kind `6`, ACK flag `1`, stream id `0`, and the
  original eight-byte opaque PING payload. Received PING ACK frames remain
  observable as received ACK frames and produce no outbound response chunk.
- The same HTTP/2 protocol-core example also covers the narrow outbound
  `RST_STREAM` send-intent. Ordinary source accepts a nonzero currently open
  stream, emits one immutable output chunk with a nine-byte frame header
  length `4`, kind `3`, flags `0`, and the selected stream id followed by the
  four-byte error-code payload, then records local reset state so a later
  stream-level `WINDOW_UPDATE` for that stream is rejected through the reset
  stream-state boundary. Stream id `0`, missing streams, closed streams,
  already reset streams, and mismatched open streams are rejected before
  output bytes are produced. Stream id and error-code values outside the
  generated binary schema encode helpers' representable ranges stay as
  `codec.encode_value_unrepresentable` failures with the generated field path.
- The same HTTP/2 protocol-core example also covers the narrow outbound
  HEADERS send-intent. Ordinary source accepts an already-encoded opaque
  header-block chunk for a nonzero currently open stream. When the
  header-block fits within the peer-advertised maximum frame size, the intent
  emits one immutable output chunk with a HEADERS frame header kind `1`,
  `END_HEADERS` set, and an optional `END_STREAM` flag, followed by the
  header-block bytes. When the header-block is larger, the same output chunk
  contains one HEADERS frame followed by as many CONTINUATION frames as needed;
  every payload chunk respects the peer-advertised maximum frame size,
  `END_HEADERS` is set only on the final frame, and optional `END_STREAM` is
  set only on the first HEADERS frame. Accepted `END_STREAM` records local
  closed-stream state so a later stream-level `WINDOW_UPDATE` for that stream
  follows the existing closed stream-state boundary. Stream id `0`, missing
  streams, closed streams, already reset streams, mismatched open streams, and
  generated frame-header representation failures are rejected before accepted
  output bytes are produced. After receiving GOAWAY,
  outbound HEADERS for an open stream id greater than the recorded
  last-stream-id are rejected with `http2.protocol.stream_after_goaway`
  before frame splitting or encode checks; HEADERS for an open stream at the
  boundary remain accepted, and stream id zero plus closed stream cases keep
  their narrower existing failures.
- The same HTTP/2 protocol-core example also covers the outbound GOAWAY
  send-intent. Ordinary source validates the selected last stream id and
  error code through a schema-declared `Http2GoawayPayloadWire` payload record
  encoded by the generated `byte_encode_<schema>` helper path, then emits one
  immutable output chunk with a nine-byte frame header length `8`, kind `7`,
  flags `0`, and stream id `0` followed by the eight-byte GOAWAY payload. An
  accepted intent
  records local graceful-shutdown state so a later peer-created HEADERS stream
  greater than the sent last stream id is rejected through the post-GOAWAY
  stream rule. Last-stream-id and error-code values outside the generated
  schema payload helper's representable ranges stay as
  `codec.encode_value_unrepresentable` failures with the generated field path
  before accepted output bytes are produced.
- Eligible direct tail-recursive user functions execute deep self-recursive
  chains without growing the host call stack for each logical step.
- Other JVM details are backend details unless this reference marks a behavior
  as an observable language boundary.

## Read When

- Core, typed IR, selected-entry reachability, and stdio ordering:
  [execution-full.md](execution-full.md#core-and-ir).
- JVM lowering support, runtime containers, file-system, network, time, and
  process intrinsics, channels, tasks, contract failures, and the class cache:
  [execution-full.md](execution-full.md#jvm-backend).

## Skip Unless Needed

- Use [commands.md](commands.md) first for command gates and user-facing
  behavior.
- Use [json-output.md](json-output.md) first for machine-readable command
  output.
