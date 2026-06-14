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
  descriptor chunk receive/send, listener creation, accept, stream read,
  stream write, timeout, and deadline waits execute outside the pure protocol
  core. Malformed received or read bytes, failed outgoing send or write event
  recording, and forced listen, accept, read, write, timeout, or deadline
  failures stop the entry as runtime failures rather than schema, codec, or
  peer protocol diagnostics.
- Stream adapter event-boundary examples use ordinary source ADT, record, and
  list values for decoded stream events and response actions. A handler
  receives an event plus explicit state and returns action intent values plus
  the next state. Channel routing uses existing `concurrency` calls; response
  actions do not perform socket writes or introduce new effect labels.
- The socket stream adapter routing example composes the existing
  fixture-backed socket calls with the source-level event/action handler
  boundary. Adapter code reads one `ByteChunk` from a `NetStream`, routes an
  ordinary event through a standard channel under `concurrency`, calls the
  plain handler, and translates ordered `SendBytes` actions into
  `net::write_chunk` calls. Handler code remains free of socket handles and
  `net` calls. The checked example is
  `examples/specification/run/socket-stream-adapter-routing/`.
- The implemented binary schema primitive execution slice decodes the
  `Http2FrameHeader` field sequence from a `ByteView`: `UInt24be`, `UInt8`,
  `UInt8`, `ReservedBits(1, 0)`, and `UInt31be`. The decoded value exposes
  ordinary `Int` fields for `length`, `kind`, `flags`, and `stream_id`.
  The reserved field is consumed and validated but is not exposed in the
  mapped record. Truncated schema fields report `schema.truncated_field`;
  invalid reserved bits report `schema.reserved_bits_mismatch`. Both carry
  byte offset and schema field path details.
- The `SchemaWidthSample` primitive decode slice consumes `UInt16be` followed
  by `UInt32be` from a `ByteView`. Both visible fields decode to ordinary
  `Int` values. Truncated fields use the same `schema.truncated_field` byte
  diagnostic shape as the frame-header slice, including byte offset, field
  path, expected count, available count, readiness, and structured byte
  preview fields.
- Generated binary schema decode helpers also support `UInt16le`, `UInt24le`,
  and `UInt32le` as little-endian unsigned primitives. They decode to
  ordinary `Int` fields, preserve structural `map to` runtime mappings, and
  use the same truncation diagnostic shape as the other exact-width
  primitives.
- Generated binary schema decode helpers support byte-aligned
  `ReservedBits(width, value)` fields up to four bytes wide as
  representation-only fields. The helper consumes the reserved bytes in
  declaration order, validates the declared fixed value, omits the field from
  the decoded value and structural mapping source values, and reports
  `schema.truncated_field` or `schema.reserved_bits_mismatch` at the reserved
  field path when the input is short or the fixed value differs.
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
- The narrow binary schema closed dispatch slice decodes
  `Dispatch(tag_field, tag => Payload, ...)` fields after the referenced tag
  field has been decoded by an earlier exact-width field in the same schema.
  Known dispatch cases consume either the selected exact-width unsigned
  payload primitive and expose an ordinary `Int` field, or the selected
  same-module or imported public nested binary schema and expose that schema's
  decoded record shape. Nested payload decode failures report the nested
  schema field path and absolute byte offset from the enclosing input. Unknown
  tags in the closed dispatch report
  `schema.dispatch_unknown_tag` at the dispatch field byte offset with schema
  field path, decoded tag field, decoded tag value, expected tags, and
  structured byte preview fields. The checked examples are
  `examples/specification/run/binary-schema-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-imported-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-dispatch-nested-failure-json/`,
  `examples/specification/run/binary-schema-imported-dispatch-nested-failure-json/`,
  `examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
  and
  `examples/specification/run/binary-schema-closed-dispatch-unknown-human/`.
- The narrow binary schema extension dispatch slice decodes
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields
  after both referenced fields have been decoded by earlier exact-width fields
  in the same schema. Known cases consume either the selected exact-width
  unsigned payload primitive or the selected same-module or imported public
  nested binary schema from the bounded payload bytes selected by
  `length_field`, then expose it as `SchemaDispatchPayload::Known(value)`.
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
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-unknown/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-unknown/`,
  and
  `examples/specification/run/binary-schema-extension-dispatch-length-human/`.
- When an eligible generated binary schema decode helper has one structural
  `map to Target` clause and the target resolves to a single record-shaped
  source type whose mapped expressions match the target field types, the
  helper returns the mapped ordinary record shape instead of the schema-local
  field shape. Mapping assignment expressions may reference decoded schema
  fields, construct records, construct ADT payloads resolved through the
  ordinary source module rules, or call one pure same-module converter
  function with one decoded schema-local field argument before assigning the
  returned value to the target field. Mapping assignment targets must name
  target fields, and every target field must be assigned once before
  execution. The implemented mapped decoded field types are exact-width unsigned primitive
  fields as `Int`, length-bounded `ByteView(length_field)` payload fields as
  `ByteView`, closed nested dispatch payload fields as the nested schema
  record shape, and extension dispatch payload fields as
  `SchemaDispatchPayload<T>`. Mapping expressions cannot call imported
  converter functions, arbitrary ordinary functions, read runtime settings,
  inspect stream state, recover from decode failures, or perform effects. The
  checked examples are
  `examples/specification/run/binary-schema-mapped-record-decode/`,
  `examples/specification/run/binary-schema-mapped-byteview-decode/`,
  `examples/specification/run/binary-schema-mapped-record-expression-decode/`,
  `examples/specification/run/binary-schema-mapped-constructor-expression-decode/`,
  `examples/specification/run/binary-schema-mapped-converter-decode/`, and
  `examples/specification/run/binary-schema-mapped-nested-dispatch-decode/`.
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
  visible exact-width unsigned primitive fields as ordinary `Int` values. A
  byte-aligned `ReservedBits(width, value)` field is representation-only: it
  is omitted from the record and the helper emits the declared fixed value in
  declaration order. A `ReservedBits(1, 0)` field immediately before a
  `UInt31be` field keeps the shared stream-identifier layout: it is omitted
  from the record and the helper emits the required zero high bit in the
  shared four-byte position.
  Closed `Dispatch(tag_field, tag => Payload, ...)` fields are eligible when
  `tag_field` names an earlier visible exact-width unsigned field and every
  case payload is an implemented exact-width unsigned primitive payload or an
  earlier same-module binary schema payload or public imported binary schema
  named through a written `use` path. The record contains the visible tag
  field and one payload field; for nested payload schemas the payload field
  uses the nested schema decoded record shape. The helper chooses the case
  from the encoded tag value, writes the selected payload in declaration
  order, and returns `Err(EncodeError("codec.dispatch_unknown_tag",
  field_path, reason))` when the tag value has no case.
  Extension-tolerant
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields are
  eligible for the same exact-width unsigned primitive, same-module nested
  binary schema, or public imported nested binary schema payload cases when
  both the tag and length fields are earlier visible exact-width unsigned
  fields. The payload record field is `SchemaDispatchPayload<T>`, where `T`
  is the selected primitive `Int` or nested schema decoded record shape.
  `Known(value)` writes the payload selected by the visible tag field.
  `Unknown(tag, payload)` writes the bounded raw bytes from the `ByteView`
  only when the visible tag value is not a known case and matches the unknown
  payload tag.
  The supplied length field remains explicit: the helper rejects values whose
  encoded payload byte count differs from the earlier length field with
  `Err(EncodeError("codec.dispatch_length_mismatch", field_path, reason))`.
  Visible tag and payload variant disagreements return
  `Err(EncodeError("codec.dispatch_mismatch", field_path, reason))`.
  The helper writes fields in declaration order into one immutable
  `ByteChunk`, using each primitive's declared byte order, and returns
  `Result<ByteChunk, EncodeError>`. `UInt16le`, `UInt24le`, and `UInt32le`
  emit little-endian bytes and use the same representability boundaries as
  their matching unsigned widths. Values outside the primitive range return
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`; nested schema encode failures keep the nested schema field path.
  `UInt31be` uses the 31-bit maximum even though it occupies four bytes.
  Unsupported non-byte-aligned reserved-bit encode shapes report
  `schema.reserved_bits_encode`.
  This slice excludes schema mappings, field-local validation, generalized
  dispatch payload schemas, other fixed fields, nested mappings, and derived
  codec encode execution for unsupported schemas.
  The checked examples are
  `examples/specification/run/binary-schema-primitive-encode/`,
  `examples/specification/run/binary-schema-primitive-encode-out-of-range/`,
  `examples/specification/run/binary-schema-reserved-bit-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-imported-closed-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode-unknown-tag/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode-out-of-range/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode-unknown/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-mismatch/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-tag-mismatch/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-out-of-range/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-dispatch-nested-encode-failure/`,
  `examples/specification/run/binary-schema-imported-dispatch-nested-encode-failure/`,
  and
  `examples/specification/check/schema-reserved-bit-encode-diagnostics/`.
- A codec declaration with a valid `derive encode` clause for the same
  eligible generated binary schema encode helper slice exposes the codec item
  name as the executable encode boundary for ordinary source calls, including
  same-module and public imported nested dispatch payload schemas already
  accepted by `byte_encode_<schema>`. The call accepts the generated helper's value
  record, invokes the schema encode helper, returns `EncodeStep<()>`, projects
  helper `Ok(ByteChunk)` output to `Encoded(List<ByteChunk>)` with one chunk,
  and projects helper `Err(EncodeError)` output to `Invalid(EncodeError)`.
  The checked examples are
  `examples/specification/run/derived-codec-encode-boundary/` and
  `examples/specification/run/derived-codec-nested-dispatch-encode-boundary/`.
  A mapped schema is rejected with `codec.encode_value_type` when the generated
  encode helper cannot accept the mapping target value type.
- A codec declaration with a valid `derive decode` clause for the same
  eligible generated binary schema decode-step slice exposes the codec item
  name as the executable decode boundary for ordinary source calls, including
  same-module nested dispatch payload schemas already accepted by
  `byte_decode_step_<schema>`. The call accepts a bounded `ByteView` and
  explicit base `ByteOffset` and returns the same `DecodeStep<T>` value as
  `byte_decode_step_<schema>`, including mapped record values,
  `NeedMore(NeedBytes(count))`, and `Invalid` without consumed bytes. The
  checked examples are
  `examples/specification/run/derived-codec-decode-boundary/` and
  `examples/specification/run/derived-codec-nested-dispatch-decode-boundary/`.
  For the implemented single structural mapping slice, `T` is the mapping
  target record shape when each assignment source has the same implemented
  decoded field type as the target field.
- A codec declaration with a valid hand-written `decode with function_name`
  clause exposes the codec item name as the executable decode boundary for
  ordinary source calls. The call accepts a bounded `ByteView` and explicit
  base `ByteOffset` and invokes the referenced same-module function.
  `NeedMore(readiness)` and `Invalid(error)` return unchanged.
  `Decoded(value, consumed)` returns unchanged when `consumed` is within the
  supplied view length; when `consumed` is outside the supplied view, the codec
  boundary returns `Invalid(DecodeError("codec.consumed_count_invalid",
  base_offset, codec_name))`.
- A codec declaration with a valid hand-written `encode with function_name`
  clause exposes the codec item name as the executable encode boundary for
  ordinary source calls. The call invokes the referenced same-module function
  with that function's parameters and returns its `EncodeStep<TState>` value
  unchanged. For the implemented single structural `map to Target` schema
  slice, the first encoder parameter remains the mapped target record shape.
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
  one.
- Executable specification cases may also assert named output `ByteChunk`
  lists through complete lowercase hex in `case.toml`. The harness checks
  stable consecutive program-output lines for the list count, chunk order,
  exact hex strings, decoded byte counts, empty lists, and zero-length chunks.
- The first ordinary-source HTTP/2 sans-I/O protocol-core example models
  chunk arrival and end-of-stream events as ADTs. Its pure decode state keeps
  undecoded suffix bytes, the next absolute byte offset, client connection
  preface state, continuation state with accumulated opaque header-block
  bytes, the active local receive-limit entry, peer-advertised SETTINGS state,
  and graceful shutdown state. It validates the
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
  decoding, zero-length SETTINGS ACK frames,
  wrong-length SETTINGS ACK payloads,
  stream id domain failures, invalid stream-state frame kinds, wrong-length
  PING, PRIORITY, GOAWAY, and `RST_STREAM` payloads, accepted PING ACK distinction,
  accepted PRIORITY dependency stream id, exclusive flag, and weight facts,
  PRIORITY self-dependency failures, peer-sent `PUSH_PROMISE` rejection,
  accepted GOAWAY last-stream-id and error-code, GOAWAY last-stream-id
  enforcement for later peer-created HEADERS streams, and accepted
  `RST_STREAM` error-code facts as typed protocol values. In the server-side
  fixture core, SETTINGS,
  PING, and GOAWAY require stream id zero; HEADERS, DATA, PRIORITY, `RST_STREAM`,
  `PUSH_PROMISE`, CONTINUATION, and stream-level `WINDOW_UPDATE` require a nonzero
  client-initiated stream id. The receive flow-control state opens an idle
  peer-created stream on an admitted HEADERS frame, counts the tracked open
  peer-created stream for the active concurrent-stream receive limit, consumes
  DATA payload length from connection and stream windows, moves the stream to
  a closed-by-peer state when accepted inbound DATA carries `END_STREAM`, accepts
  connection-level and open-stream `WINDOW_UPDATE` increments, applies
  received `SETTINGS_INITIAL_WINDOW_SIZE` deltas to the tracked open-stream
  receive-window credit, and keeps wrong-length, idle-stream, zero,
  closed-by-peer stream, reset-stream, concurrent-stream-limit,
  header-list-size, negative-credit DATA, and overflow cases as typed
  protocol failures.
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
- The first HPACK fixture-codec examples model HPACK as an imported ordinary
  source module, not as schema syntax. The fixture module accepts a small
  deterministic set of header-block byte fixtures, returns ordinary header-list
  data plus the next immutable fixture state, and projects unsupported fixture
  input through `hpack.fixture.unsupported_header_block`. That diagnostic path
  is distinct from `schema.*`, `http2.protocol.*`, and `http2.peer_limit.*`
  ids; the HTTP/2 core still owns the local
  `http2.peer_limit.header_list_size_exceeded` receive-limit boundary after
  fixture decoding.
- The same example keeps outbound DATA send-intent flow control separate from
  inbound receive limits. Received `SETTINGS_MAX_FRAME_SIZE` constrains DATA
  payloads this endpoint sends, received `SETTINGS_INITIAL_WINDOW_SIZE`
  supplies the outbound stream credit for the peer-owned stream window, valid
  DATA intents consume outbound connection and stream credit, and oversized or
  over-window DATA intents are rejected before credit changes.
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
  GOAWAY send-intent. Ordinary source validates the selected last stream id
  through the generated `UInt31be` payload helper and the error code through
  the generated `UInt32be` payload helper, then emits one immutable output
  chunk with a nine-byte frame header length `8`, kind `7`, flags `0`, and
  stream id `0` followed by the eight-byte GOAWAY payload. An accepted intent
  records local graceful-shutdown state so a later peer-created HEADERS stream
  greater than the sent last stream id is rejected through the post-GOAWAY
  stream rule. Last-stream-id and error-code values outside the generated
  payload helper's representable ranges stay as
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
