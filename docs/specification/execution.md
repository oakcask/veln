# Execution Boundary

This page routes implemented execution facts. Open
[execution-full.md](execution-full.md) only when a short route here is not
enough.

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
- Standard byte chunk and byte view helpers execute as pure prelude runtime
  operations and return immutable byte values or `Result` failures for invalid
  values, invalid compact hex fixture text, out-of-bounds counts and ranges,
  fixed-width unsigned read truncation, schema fixed-field mismatches, bounded
  view slicing, and conversion overflow.
- `ByteView`, `ByteChunk`, `StreamInput`, `DecodeStep<T>`,
  `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`, and `EncodeError`
  values execute as ordinary immutable source-visible values.
- `byte_chunks_produce(chunks, budget)` is a pure outgoing chunk helper. It
  produces only whole `ByteChunk` values that fit within the supplied
  `ByteCount`, reports the produced byte count, preserves chunk order, and
  returns the unproduced suffix for a later call.
- `net` and `time` calls are host runtime boundaries. Fixture-backed and
  production-loopback transport paths preserve the same source-visible result
  shapes while keeping socket, deadline, cancellation, and monotonic-clock
  work outside pure protocol code. `NetListener` handles expose local
  endpoint text through `net::listener_local_addr` before accept work without
  exposing host socket handles, closing the listener, or changing later
  accepted streams. Accepted and connected `NetStream` handles expose local
  and peer endpoint text through `net::stream_local_addr` and
  `net::stream_peer_addr` without exposing host socket handles or changing
  stream ownership. Production-loopback cases can preserve more than one
  configured read chunk for one accepted stream, and `net::connect` can return
  a deterministic client-side loopback stream with the same read, write, and
  close lifecycle; each chunk is observed by source as a separate read result
  before clean end. A source-visible production listen/connect lifecycle can
  also use one address value for `net::listen` and `net::connect`, accept the
  paired server stream, exchange a byte chunk across the two owned stream
  handles, close both handles, and then observe clean listener end before
  closing the listener. Production-loopback write-side shutdown records output
  half-close before full stream close and makes later writes runtime transport
  failures; production-loopback read-side shutdown makes later optional stream
  reads report clean end while preserving write-side ownership until explicit
  write shutdown or stream close. Production-loopback stream state inspection
  observes read-capable, write-capable, and closed status as source-visible
  Bool values before half-close, after each half-close, and after full close
  without consuming the `NetStream` handle; later transport operations through
  a stale closed handle fail as runtime transport failures. The
  cancellable receiver-list channel-first adapter observes cancellation as an
  ordinary routed, timed-out, or cancelled source outcome before producing
  adapter actions, instead of adding another fixed route-count execution
  shape. The
  multi-event adapter task-helper routing case preserves adapter-owned trace
  identity and event sequence while routing those source-visible stream events
  through channel and an adapter-owned task helper before ordered chunk-list
  writes. A companion per-stream handler-failure case returns an ordinary
  handler failure value from the task boundary, skips later response writes
  for that stream, closes the accepted stream, and then observes deterministic
  listener end.
  `stream_adapter_drain_actions` is the standard adapter-level stream routing
  helper: it drains one accepted production stream into ordered
  `StreamAdapterAction` values through channel-routed `StreamInput` values,
  writes only `SendBytes` chunks via `net::write_chunks`, and leaves stream
  close and listener-end observation to the adapter caller.
  That helper owns the `concurrency` boundary and calls a pure event/action
  handler.
  `stream_adapter_accept_loop` owns the listener-level adapter boundary: it
  repeatedly accepts streams until clean listener end, drains each accepted
  stream through `stream_adapter_drain_actions`, projects ordered `SendBytes`
  chunks through `net::write_chunks`, closes each stream, and closes the
  listener after clean end. Forced accept, read, write, stream close, and
  listener close failures remain runtime transport failures at the same host
  boundaries.
  `stream_adapter_drain_actions_until_cancellable` keeps that drain, channel,
  and pure handler boundary, then writes only projected `SendBytes` chunks via
  `net::write_chunks_until_cancellable` with the supplied `Deadline` and
  `CancelToken`. It returns ordinary write outcome values for completion,
  deadline expiry, or cancellation, while host write failures remain runtime
  transport failures. The
  multi-cycle routing case accepts more than one production stream from one
  listener and preserves repeated read, route, ordered write, close, and clean
  listener-end observations without exposing socket handles to handlers.
  Forced production read failure on the multi-chunk routing path stops after
  production accept and before later routing, response writes, stream close,
  or clean listener end.

## Binary Schemas

- Every exact-width unsigned primitive from `uint8` through `uint64be` and
  `uint64le` decodes to `Int` and accepts `Int` on encode. This holds for
  direct, repeated, anonymous-record, nested, closed-dispatch,
  extension-dispatch, explicit-operation, and derived-helper paths.
  Endianness changes byte representation only; bit zero remains the decoded
  value's least-significant bit in both byte orders.

- Explicit schema decode expressions lower to the generated decode-step
  boundary for the referenced eligible binary schema. They use the supplied
  `ByteView` as bounded input and the supplied `ByteOffset` for consumed-count
  and diagnostic offset accounting, returning `DecodeStep<T>` for the
  schema-local visible record shape. Public schema aliases lower through the
  same generated boundary as the aliased schema. The HTTP/2 frame header
  schema exposes the visible record fields `length`, `kind`, `flags`, and
  `stream_id` through this path; its representation-only reserved bit is
  omitted from the decoded record and still reports
  `schema.reserved_bits_mismatch` at `Http2FrameHeaderWire.stream_reserved`
  when set. When bounded input ends at a field boundary before the next field
  starts, explicit decode returns `NeedMore(NeedBytes(total_schema_width))`;
  when bounded input ends after at least one byte of the current field has
  been consumed, explicit decode returns `Invalid(DecodeError(...))` with
  `schema.truncated_field`, the field path, and the explicit base offset.
  Hand-written decode boundaries validate `Decoded(value, consumed)` against
  the supplied `ByteView`; a negative consumed count or a count larger than the
  view length returns `Invalid(DecodeErrorWithReason(...))` with
  `codec.consumed_count_invalid`, the supplied view length, the actual
  consumed count, and no retryable readiness.
- Explicit schema encode expressions lower to the generated encode boundary
  for the referenced eligible binary schema. They typecheck the supplied value
  against the schema-local visible record shape and return
  `Result<ByteChunk, EncodeError>`. Public schema aliases lower through the
  same generated boundary as the aliased schema.
- Compatibility generated binary schema decode helpers read fields in
  declaration order and return the schema-local visible record shape. They are
  retained for old fixtures and runtime adapter coverage, not as the public
  source surface for applying schemas.
- Consecutive visible `UInt1` through `UInt7` fields whose widths complete
  one, two, three, four, five, six, seven, or eight big-endian bytes are
  packed into the shared storage unit in declaration order. Decode exposes
  ordinary `Int` fields, encode accepts the same schema-local visible record,
  and truncation reports `schema.truncated_field` at the packed visible field
  path corresponding to the missing input position. The eight-byte boundary is
  checked by
  `examples/specification/run/binary-schema-packed-visible-eight-byte-decode-encode/`,
  `examples/specification/run/binary-schema-packed-visible-eight-byte-truncated-json/`,
  and
  `examples/specification/run/binary-schema-packed-visible-eight-byte-encode-out-of-range/`.
- Format-neutral generated decode helpers for schemas without a `format`
  clause accept a schema-local visible record shape and return
  `Result<T, String>`. The helper returns the supplied record on success and
  is limited to recursive format-neutral visible shapes made from scalar
  leaves, anonymous record fields, `Option<T>`, `List<T>`, `Vec<T>`, and
  `Dict<String, T>`. `Result<Ok, Err>` is supported when both payloads are
  recursive format-neutral visible shapes. Same-module source ADTs and public
  imported source ADTs referenced through written `use` paths are supported in
  the same positions when every constructor payload is a recursive
  format-neutral visible shape; the helper preserves the source ADT value
  shape through the pass-through boundary.
- Format-neutral generated encode helpers for schemas without a `format`
  clause accept a schema-local visible record shape and return
  `Result<T, String>` when every field is a recursive visible shape made from
  `Int`, `Bool`, `Float`, and `String` leaves, anonymous records, `Option<T>`,
  `List<T>`, `Vec<T>`, `Dict<String, T>`, `Result<Ok, Err>`, and eligible
  same-module or public imported source ADTs. Every recursively visited child
  or constructor payload must also be eligible; container depth is not
  otherwise limited.
  The helper returns the supplied record on success and does not produce binary
  bytes.
- Repeated fields written as `[Payload; count]` normalize to the same generated
  decode and encode helper behavior as `Repeat(count, Payload)`, with the
  payload before `;` and the count expression after it. The count expression
  uses the same earlier-field and arithmetic forms accepted by `Repeat`.
  Lowercase exact-width `uint...` payloads written in legacy
  `Repeat(count, Payload)` fields normalize to the same generated decode and
  encode helper behavior as the matching canonical repeated-field payload.
  Lowercase `uint... reserves <value>` payloads written in legacy
  `Repeat(count, Payload)` fields validate the declared reserved value at each
  repeated element during decode, emit that value once per count during encode,
  append the repeated element index to mismatch and truncation diagnostics, and
  do not expose a visible list field.
  Repeated `ByteView(left_length - right_length)` payloads expose
  `List<ByteView>` and report truncation with the repeated element index.
  Repeated nested payloads may name a same-module recursive binary schema when
  that nested schema already exposes a finite same-module recursive dispatch
  helper shape through a length-bounded field and a non-recursive primitive base
  case. The repeated field exposes `List<NestedRecord>` using that finite
  nested schema-local visible record shape, and failures inside a recursive
  repeated element preserve the repeated field path, element index, nested
  schema names, and failing nested field path. The checked cases are
  `examples/specification/run/binary-schema-recursive-repeat-nested-decode/`
  and
  `examples/specification/run/binary-schema-recursive-repeat-nested-failure-json/`.
- Direct nested binary schema fields name an eligible same-module or public
  imported nested binary schema, consume that nested schema in place, and
  expose the nested schema-local visible record at the field.
- Anonymous record fields in `format binary` schemas consume their exact-width
  unsigned primitive leaves in source order and expose the nested anonymous
  record shape at the field. Anonymous records may recurse when all leaves are
  exact-width unsigned primitives, and a record may contain sibling nested
  anonymous record fields at the same level. Truncation inside the anonymous
  record preserves the outer field segment and appends every anonymous record
  field segment down to the failed primitive. The checked cases are
  `examples/specification/run/binary-schema-anonymous-record-decode/`,
  `examples/specification/run/binary-schema-anonymous-record-truncated-json/`,
  `examples/specification/run/binary-schema-nested-anonymous-record-decode/`,
  `examples/specification/run/binary-schema-nested-anonymous-record-truncated-json/`,
  `examples/specification/run/binary-schema-sibling-nested-anonymous-record-decode/`,
  `examples/specification/run/binary-schema-sibling-nested-anonymous-record-truncated-json/`,
  `examples/specification/run/binary-schema-recursive-anonymous-record-decode/`,
  and
  `examples/specification/run/binary-schema-recursive-anonymous-record-truncated-json/`.
- Generated binary schema encode helpers accept anonymous record fields whose
  leaves are exact-width unsigned primitives, write leaves in declaration
  order with each primitive's byte order, and preserve nested anonymous record
  field segments in `schema.encode_value_unrepresentable` paths. The checked
  cases are
  `examples/specification/run/binary-schema-anonymous-record-encode/`,
  `examples/specification/run/binary-schema-anonymous-record-encode-out-of-range-json/`,
  and
  `examples/specification/check/binary-schema-anonymous-record-encode-boundary/`.
- Dispatch payload cases written with lowercase exact-width `uint...` primitive
  spelling normalize to the same generated decode and encode helper behavior as
  compatible upper-case exact-width payload spelling.
  Byte-aligned lowercase `uint... reserves <value>` dispatch payloads validate
  the fixed payload bytes during decode, emit those bytes during encode, and
  expose `()` as the payload value. Direct subbyte spellings from
  `uint1 reserves 0` through `uint7 reserves 127` consume one payload byte
  when the reserved value fits the declared width, validate the high-order
  payload bits, emit the declared reserved value in that storage byte during
  encode, and also expose `()` as the payload value.
- Closed and extension dispatch payload cases may name an eligible same-module
  or public imported nested binary schema. Nested payload schemas expose their
  schema-local visible record shapes through the same generated decode and
  encode helper boundary as ordinary nested schema fields, including
  length-bounded `ByteView(left_length / right_length)` fields whose operands
  are earlier visible `Int` fields in the nested payload schema.
- Closed dispatch payload schemas may decode bounded repeated fields whose
  payload is an eligible nested binary schema. Truncation inside a repeated
  nested payload preserves the parent dispatch field path, the selected nested
  payload schema, the repeated element index, and the nested field path. The
  checked cases are
  `examples/specification/run/binary-schema-dispatch-nested-repeat-decode/`
  and
  `examples/specification/run/binary-schema-dispatch-nested-repeat-truncated-json/`.
- Same-module recursive dispatch payload cases expose a finite primitive
  payload shape when the recursive dispatch field is length-bounded and has a
  non-recursive primitive base case. Decode helpers collapse recursive known
  payload chains to that primitive payload value. Encode helpers accept the
  same schema-local visible shape and can encode the primitive base case while
  preserving the usual dispatch length checks. The checked success case is
  `examples/specification/run/binary-schema-recursive-dispatch-decode-encode/`;
  the checked missing-base rejection case is
  `examples/specification/run/binary-schema-recursive-dispatch-rejected/`.
- Representation-only fields such as supported `ReservedBits(width, value)`
  and lowercase `uint... reserves <value>` layouts are validated and omitted
  from the decoded record.
- A direct `ReservedBits(width, value)` followed by `UInt8` uses one
  big-endian storage group when the width is positive and non-byte-aligned,
  the value fits the width, and the group with trailing padding fits in at
  most eight bytes. Decode and encode omit the reserved field, expose the
  visible byte, consume the full padded group, and report truncation or
  `schema.reserved_bits_mismatch` at the declared field paths. The checked
  boundary cases are
  `examples/specification/run/binary-schema-general-reserved-byte-prefix-decode-encode/`
  and
  `examples/specification/run/binary-schema-general-reserved-byte-prefix-json/`.
- Generated `validate_<schema>` helpers accept the schema-local decoded record
  shape and check field-local `where` predicates plus the single schema-level
  `validate` predicate when present.
- Generated binary schema encode helpers accept the schema-local visible
  record shape, validate field-local and representation constraints, and write
  bytes through the declared schema layout. Lowercase reserved-bit fields emit
  their declared values and are omitted from the input record like compatible
  `ReservedBits(width, value)` fields.
- Direct visible big-endian `UInt16be`, `UInt24be`, `UInt31be`, and
  `UInt32be` compatibility helper parity is checked by
  `examples/specification/run/binary-schema-big-endian-widths-decode-encode/`
  and
  `examples/specification/run/binary-schema-big-endian-widths-encode-out-of-range/`.
- Five-byte `UInt40be` and `UInt40le` compatibility helper coverage is
  checked by
  `examples/specification/run/binary-schema-u40-widths-encode/`,
  `examples/specification/run/binary-schema-u40-widths-encode-out-of-range/`,
  and
  `examples/specification/run/binary-schema-u40-widths-truncated-json/`.
  Six-byte `UInt48be` and `UInt48le` compatibility helper coverage is checked
  by `examples/specification/run/binary-schema-u48-widths-encode/`,
  `examples/specification/run/binary-schema-u48-widths-encode-out-of-range/`,
  and
  `examples/specification/run/binary-schema-u48-widths-truncated-json/`.
  Source-visible seven-byte `byte_read_u56_be`, `byte_read_u56_le`,
  `byte_write_u56_be`, and `byte_write_u56_le` helper behavior is checked by
  `examples/specification/run/binary-byteview-u56-helpers/`.
  Seven-byte `UInt56be` and `UInt56le`, plus eight-byte `UInt64be` and
  `UInt64le`, compatibility helper parity is checked by
  `examples/specification/run/binary-schema-u56-widths-encode/`,
  `examples/specification/run/binary-schema-u56-widths-encode-out-of-range/`,
  `examples/specification/run/binary-schema-u56-widths-truncated-json/`,
  `examples/specification/run/binary-schema-u64-widths-encode/`,
  `examples/specification/run/binary-schema-u64-widths-encode-out-of-range/`,
  `examples/specification/run/binary-schema-u64-widths-integer-out-of-range-human/`,
  `examples/specification/run/binary-schema-u64-widths-integer-out-of-range-json/`,
  `examples/specification/run/binary-schema-u64le-widths-integer-out-of-range-json/`,
  and `examples/specification/run/binary-schema-u64-widths-truncated-json/`.
- Representation-local generated schema encode failures that cannot write a
  supplied value, repeat count, or length-bounded `ByteView` use
  `schema.encode_value_unrepresentable` while preserving the existing
  `EncodeError` field path and reason shape. Hand-written codec
  `EncodeError(...)` values may still use codec-owned ids.
- Representation-local generated schema dispatch encode failures use
  `schema.dispatch_unknown_tag`, `schema.dispatch_length_mismatch`, and
  `schema.dispatch_mismatch` for unknown closed-dispatch tags, extension
  dispatch payload length mismatches, and extension dispatch tag/payload
  mismatches. Compatibility-only hand-written `EncodeError(...)` values may
  still use the corresponding `codec.dispatch_*` ids.
- Projection between a schema-local record and a domain value is ordinary Veln
  source at the caller or schema-operation boundary. The checked schema-local projection
  case is
  `examples/specification/run/binary-schema-local-projection-boundary/`.
- Schema-level `map to` clauses are rejected by the parser before execution.
  The checked rejection cases are
  `examples/specification/check/schema-map-to-rejected/`,
  `examples/specification/check/schema-map-to-selector-rejected/`, and
  `examples/specification/check/schema-map-to-inverse-rejected/`.

## Codecs

- Source-level `codec` and `pub codec` declarations are rejected before
  execution. Executable decode and encode entry points are ordinary functions
  or explicit schema operation expressions.
- Compatibility-only runtime diagnostic ids under `codec.*` remain part of
  runtime diagnostic vocabulary where existing runtime values use them.
- The source-visible HPACK static decoder accepts static indexed fields,
  bounded static-name literal-without-indexing fields, bounded static-name
  literal-with-indexing fields, and bounded static-name literal-never-indexed
  fields for names resolved through the HPACK static table metadata, including
  raw visible-ASCII values and bounded Huffman-marked literal values decoded
  through the HPACK static Huffman table. The checked static boundary includes
  visible ASCII, line feed, single-byte `hpack-byte-*` labels, and multi-byte
  `hpack-bytes-*` labels across the static-name literal forms under
  `examples/specification/run/hpack-static-codec-boundary/`. Stateful
  HTTP/2 request and response decoding also accept `content-length` through the
  static-name literal forms checked by
  `examples/specification/run/http2-protocol-core/` when no later fixture
  dynamic-table reuse is observed. The decoded values feed the same
  header-list validation and content-length body-accounting paths as fixture
  header lists. Stateful HTTP/2 request decoding also validates static-name
  `:scheme` literal values against the same request header-list rule as
  fixture-marked values, accepting raw `http` and `https` values plus the
  checked Huffman-marked `https` value and rejecting other visible ASCII
  values with `scheme_value_not_http_or_https` on completed HEADERS and final
  CONTINUATION paths. The same receive path routes the checked
  Huffman-marked `:path: test` static-name literal through completed HEADERS
  and final CONTINUATION paths before fixture fallback, so the decoded visible
  ASCII value reaches the ordinary request header-list path. It also validates
  source-visible
  static-name `:authority` literal values through the existing request
  header-list path, accepting checked visible ASCII authority values and
  rejecting the checked invalid visible ASCII value with
  `authority_value_invalid` on completed HEADERS and final CONTINUATION paths.
  Ordinary `CONNECT` request validation uses a distinct pseudo-header shape:
  `:method: CONNECT` requires a non-empty `:authority` and rejects any
  present `:scheme` or `:path`. The same validation runs after a completed
  HEADERS block and after the final CONTINUATION block. Rejection preserves
  the carried HPACK, stream, and output state at the request-header failure
  boundary.
  Stateful HTTP/2 response decoding validates `:status` pseudo-header values
  after fixture decode and after source-visible HPACK static-name literal
  decode. Accepted response lists keep exactly three ASCII decimal digits, and
  empty, short, long, or non-decimal values fail with
  `status_value_invalid` through the response header-list diagnostic on
  completed HEADERS and final CONTINUATION paths.
  Stateful HTTP/2 response decoding also accepts static-indexed
  `cache-control` and `content-type` entries after a static-indexed `:status`
  through completed HEADERS and final CONTINUATION paths in the same checked
  protocol-core case. The HTTP/2 protocol-core fixture also exposes a
  source-visible HPACK Huffman encode boundary for outbound checked values:
  `encode_hpack_huffman_payload` accepts supported source-visible strings,
  `encode_hpack_huffman_payload_bytes` accepts bounded byte chunks, and both
  return only the encoded Huffman payload bytes with EOS padding. Unsupported
  source-visible strings stay on the ordinary
  `Result<HpackFixtureFailure>` path instead of a runtime diagnostic.
  Stateful HTTP/2 header-block decoding routes supported
  static-name literal-with-indexing fields through the source-visible static
  decoder, inserts the decoded name/value pair into the carried HPACK dynamic
  state, and resolves a following `0xbe` dynamic indexed field from that
  state. Unsupported literal-with-indexing forms still fall back to the HPACK
  fixture boundary when the checked source-visible decoders do not own the
  form. The checked receive path inserts a literal-with-indexing
  `:path: /target` entry, resolves a following `0xbe` dynamic indexed field,
  and preserves the focused dynamic-index failure after the carried table no
  longer contains a matching entry.
- The source-visible `hpack_dynamic_core` boundary accepts the checked dynamic
  indexed header-field representation when the caller supplies a bounded
  dynamic table carrying the referenced entries. The checked boundary decodes
  `0xbe` to the newest carried entry and `0xbf` to the next older carried
  entry. It also decodes saturated seven-bit indexed representations
  `0xff 0x00` and `0xff 0x80 0x00` as HPACK index `127`, resolving dynamic
  table index `65` when the bounded carried table contains that retained
  entry. The boundary
  advances the dynamic-core decode count after each accepted decode and
  reports `hpack.fixture.dynamic_index_out_of_range` with requested dynamic
  index and entry-count facts when an indexed field asks past the carried
  table without advancing state, including the checked multi-continuation
  out-of-range representation `0xff 0x80 0x01`. HTTP/2 completed HEADERS
  decoding tries this source-visible dynamic indexed boundary before fixture
  fallback for accepted and out-of-range dynamic indexed fields. The same
  ordinary-source boundary exposes a bounded HPACK integer core for the
  checked indexed-field, table-size update, and string-literal-length prefix
  widths. The standalone boundary decodes those integer shapes, encodes the
  same bounded shapes, and reports non-terminating continuations through the
  focused `hpack.integer.malformed` fact with byte offset, prefix width,
  observed byte count, observed first byte, bounded preview count, and module
  name under `examples/specification/run/hpack-fixture-codec-boundary/`.
  At the checked HPACK fixture boundary, up to two consecutive dynamic
  table-size updates at the start of one header block are decoded and applied
  in wire order. A following supported header field is decoded with the final
  table capacity and updated immutable state. Reducing the capacity evicts entries
  immediately, and a later increase in the same block does not restore them;
  the checked dynamic-index lookup observes that wire-order result. Completed
  HEADERS and final CONTINUATION paths use the same boundary. Each requested
  size is checked against the local receive limit before any next HPACK state
  is installed. A third leading update is outside the bounded fixture
  vocabulary and keeps the focused placement diagnostic;
  the first excessive update reports
  `http2.peer_limit.header_table_size_exceeded`, while a malformed update or
  an update after a header field keeps the existing focused HPACK diagnostic.
  Failed blocks leave the carried HPACK state unchanged. The executable cases
  are `examples/specification/run/hpack-fixture-codec-boundary/` and
  `examples/specification/run/http2-protocol-core/`.
  The same ordinary-source boundary exposes
  HPACK dynamic entry size as header-name byte count plus header-value byte
  count plus `32`, preserves immutable state while inserting newest-first
  dynamic entries, evicts oldest entries after insertion or table-size
  reduction including reduction to a zero-size table, keeps evicted entries
  absent when the table size later expands, accepts later insertion into a
  reexpanded table, and clears the table when an inserted entry is larger than
  the supplied table-size limit. The same
  boundary accepts the checked static-name literal-with-indexing block
  `content-type: text`, returns the decoded header entry and wire size, inserts
  it into the immutable dynamic-core state using the same accounting rule, and
  resolves a following `0xbe` dynamic indexed field from the inserted entry.
  It also accepts checked raw visible-ASCII literal-name fields across the
  literal-without-indexing, literal-with-indexing, and literal-never-indexed
  forms when the value is raw visible ASCII or a bounded Huffman-marked value
  accepted by the existing checked HPACK Huffman boundary. Only
  literal-with-indexing mutates the immutable dynamic table; the other two
  forms advance the decode count without adding an entry. The checked raw
  literal-name insertion is then reused through the existing dynamic indexed
  path. HTTP/2 completed HEADERS and final CONTINUATION decoding route
  accepted raw literal-name fields through this source-visible boundary before
  fixture fallback, while unsupported or malformed forms keep the existing
  fixture fallback diagnostics, including focused malformed Huffman padding
  projection. The same boundary accepts checked dynamic-name literal receive
  forms whose header name comes from the carried bounded dynamic table.
  Literal-without-indexing and literal-never-indexed return the decoded header
  while retaining the existing table entry for later `0xbe` reuse.
  Literal-with-indexing inserts the fresh name/value pair using the same
  bounded accounting rule, then exposes the inserted entry through `0xbe`
  while retaining the older entry as `0xbf` when the table has room. These
  dynamic-name forms accept raw visible-ASCII values and bounded
  Huffman-marked values decoded by the existing checked HPACK Huffman boundary;
  malformed Huffman padding reports the focused
  `hpack.fixture.malformed_huffman_padding` shape at the dynamic-core boundary
  and through HTTP/2 fixture projection. The checked cases are
  `examples/specification/run/hpack-fixture-codec-boundary/` and
  `examples/specification/run/http2-protocol-core/`.
  The production inbound boundary replaces the fixed two-field decode result
  with a recursive ordinary header-field value. It decodes arbitrary finite
  sequences of the supported indexed and literal representations in wire
  order and consumes any finite sequence of legal leading dynamic-table size
  updates before the first field. Updates are applied in wire order, and the
  first field observes the final accepted capacity. The checked standalone
  transition requires the active local receive limit, and every update is
  checked against that limit before a next state is exposed. Malformed,
  excessive, or post-field updates return the focused existing diagnostic
  families at the offending update's absolute HPACK byte offset with the
  inspected suffix. Any failure returns no header list or committed next
  state, leaving the input state reusable. Complete HEADERS and
  `PUSH_PROMISE` blocks plus final CONTINUATION assembly share this production
  boundary. The executable protocol-core case checks zero, one, two, and
  more-than-two leading updates, late-sequence failures, input-state reuse,
  and the existing mixed-field, validation, and carried-state behavior.
  Production literal values are immutable octets rather than visible-ASCII or
  fixture-label strings. Raw and Huffman-decoded values retain their exact
  bytes across all literal indexing policies, static and dynamic name paths,
  dynamic-table accounting, insertion, eviction, and indexed reuse. Header
  names remain validated strings; request, response, and trailer rules convert
  only `:method`, `:scheme`, `:path`, `:authority`, `:status`, `te`, and
  `content-length` to their required ASCII shapes, while ordinary values stay
  opaque. The checked case covers non-visible raw and Huffman values through
  standalone decode, completed HEADERS, and final CONTINUATION assembly.
- The checked HTTP/2 receive state carries pending header-block continuation
  assembly as connection decode state. A HEADERS frame without `END_HEADERS`
  records the owning stream id, starting frame kind, starting byte offset, and
  accumulated opaque header-block bytes without emitting a completed header
  event. Same-stream CONTINUATION frames append payload bytes until
  `END_HEADERS` completes the combined header block for HPACK decoding. A
  different frame kind, different stream id, or input end while this assembly
  is pending reports protocol-owned continuation context with the pending
  stream, start frame, start offset, accumulated byte count, and rule
  provenance. The checked coverage includes the single and multiple
  CONTINUATION success paths, wrong frame kind, wrong stream id, and
  end-of-input while pending under `examples/specification/run/`.
- The source-visible HPACK fixture encoder accepts exact static-table
  name/value pairs with fixed values from the finite HPACK static table as
  indexed-field bytes under
  `examples/specification/run/hpack-fixture-codec-boundary/`, including
  checked request pseudo-header, response pseudo-header, and ordinary-header
  entries. Non-exact values for known static names remain fixture encode
  failures. Outbound HTTP/2 HEADERS and server-side `PUSH_PROMISE` fixture
  paths route supported static-indexed header lists through the same helper
  before fixture fallback and frame splitting. The
  same focused HPACK fixture boundary checks a source-visible static-name
  literal-without-indexing helper: it resolves header names through the
  finite HPACK static table metadata, emits raw visible-ASCII values for
  known static names such as `:method`, `:path`, and `server`, and leaves
  non-static names on the HPACK fixture encode-failure path. The
  same focused HPACK fixture boundary case checks payload-only Huffman
  encoding directly for a supported string, bounded byte input, and an
  unsupported string returned as a fixture encode failure. The
  source-visible HPACK fixture encoder also accepts the checked outbound
  dynamic-name literal-without-indexing, literal-with-indexing, and
  literal-never-indexed slices under
  `examples/specification/run/hpack-fixture-codec-boundary/` and routes the
  returned encode state through outbound HEADERS and
  server-side `PUSH_PROMISE` framing for the literal-with-indexing form in
  `examples/specification/run/http2-protocol-core/`. All three dynamic-name
  forms accept values from the checked source-visible Huffman encoder. The
  literal-with-indexing form inserts the decoded name and value for later
  `0xbe` reuse; the other forms leave the prior dynamic entry reusable.
  Missing dynamic-name state and unsupported Huffman input remain fixture
  encode failures before a HEADERS block is emitted.
  The same fixture encoder
  accepts a checked ordinary new-name literal-with-indexing header list for
  visible-ASCII field-name and value pairs that pass the outbound ordinary
  header-name validation boundary. Encoding `x-trace: ok` first emits raw
  literal-with-indexing bytes and returns a dynamic-table entry; encoding the
  same header list from that returned state emits dynamic indexed byte `0xbe`
  through the outbound HEADERS path. Unsupported ordinary names remain HPACK
  fixture encode failures before HEADERS bytes are emitted. The same fixture
  encoder accepts checked new literal names from the source-visible Huffman
  encoder across literal-without-indexing, literal-with-indexing, and
  literal-never-indexed. Each form composes the Huffman name with raw and
  Huffman literal values. Only literal-with-indexing inserts the decoded pair
  for later `0xbe` reuse; the other forms retain an empty dynamic table, and
  unsupported Huffman-name input returns a fixture failure without output or
  mutation of the carried state. The checked indexed block and its returned
  state are routed through outbound HEADERS. The same fixture
  encoder exposes an ordered header-field list that accepts any finite number
  of already-validated names and immutable octet values, carried immutable
  state, and the
  active peer-advertised dynamic-table capacity. It selects
  exact static indexed, exact dynamic indexed, static-name literal,
  dynamic-name literal, then new-name literal in that order. The selected
  literal policy is literal-with-indexing, so only the three literal choices
  insert. Each literal name and octet value independently compares its
  complete raw and HPACK Huffman encodings, selects Huffman only when it is
  smaller, and keeps
  raw encoding on ties without interpreting value bytes as text. Exact static
  indexed selection requires the octets to equal the fixed static-table value;
  exact dynamic-indexed fields retain their representation priority. Octet
  values are preserved through static-name, dynamic-name, and new-name
  literals, dynamic-table byte accounting, insertion, eviction, and reuse.
  Multi-field and carried-state blocks
  check static and dynamic exact reuse alongside new-name and dynamic-name
  insertion. Outbound
  request and response HEADERS and server-side `PUSH_PROMISE` route the
  ordered list through the same frame splitting and CONTINUATION paths as
  pre-encoded blocks. Reduced-capacity insertion uses the existing eviction
  rules. When the peer lowers its advertised capacity, the next ordered block
  starts with a table-size update and evicts retained entries before encoding
  fields; an encoder-selected size below a later increased peer limit remains
  valid. Explicit table-size updates can also evict retained entries or clear
  the table.
  The checked outbound HEADERS cases cover non-visible raw octets, a smaller
  Huffman octet value, a raw tie,
  mixed Huffman-name/raw-value selection, Huffman dynamic-name values, and
  later exact dynamic reuse. Invalid names, unsupported encodings, and capacity
  mismatches return no frame bytes and expose no partially updated HPACK or
  protocol state; the reusable input state remains unchanged. The same fixture
  encoder observes the current outbound dynamic-table capacity after a checked
  table-size update: a later literal-with-indexing entry larger than the
  capacity is not retained for dynamic-index reuse, and a zero-capacity update
  clears retained entries and keeps later literal-with-indexing HEADERS
  encodes on the literal path. An entry that fits the reduced capacity is
  retained and reused through the outbound HEADERS path.
- The checked HTTP/2 protocol core represents validated real streams with an
  ordinary `StreamId` value backed by `Int` and represents a frame target with
  `StreamRef`, which distinguishes the connection stream from a real stream.
  Binary frame schemas still decode `UInt31be` fields to `Int`; the receive
  boundary constructs the domain values before stream-state admission.
  Client-initiated and server-initiated constructors reject zero, values above
  the 31-bit HTTP/2 maximum, and endpoint-invalid parity. The connection
  reference cannot satisfy the real-stream branch, while a real `StreamId`
  cannot satisfy the connection-only branch. Existing failures retain
  `http2.protocol.invalid_stream_id`, its focused required-domain fact,
  endpoint role, active state, and rule provenance. The executable evidence is
  under `examples/specification/run/http2-protocol-core/`.
- The same receive core retains the greatest accepted client-initiated stream
  id independently of the currently open stream set. A HEADERS frame that
  would create a new peer-created stream must use an id greater than that
  retained value; reuse of a tracked stream id continues through its existing
  lifecycle rules. Closing or resetting the greatest stream does not lower the
  retained value. Frame-size, stream-id-domain, payload, HPACK, and completed
  header-list validation run before this ordering check. For an otherwise
  valid untracked stream, ordering runs before the concurrent-stream limit and
  GOAWAY admission checks. Rejection uses
  `http2.protocol.peer_stream_id_not_increasing` and preserves stream,
  flow-control, HPACK, shutdown, and retained high-water state apart from
  ordinary input-consumption state. The broad checked case is
  `examples/specification/run/http2-protocol-core/`; focused human and JSON
  projections are under
  `examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-human/`
  and
  `examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-json/`.
- In the client receive fixture, the same connection-level high-water state
  retains the greatest accepted server-initiated promised stream id. A valid
  `PUSH_PROMISE` that would reserve an untracked promised stream must increase
  that value; tracked promised-stream reuse remains on its lifecycle failure
  path. Promised stream id domain checks, local disable-push policy, associated
  stream state, HPACK decode, and promised request-header validation precede
  ordering. Rejection preserves promised-stream, flow-control, HPACK,
  settings, shutdown, and high-water state apart from ordinary consumed input,
  and a rejected higher id remains eligible for a valid retry. The broad
  checked case is `examples/specification/run/http2-protocol-core/`; focused
  client human and JSON projections are under
  `examples/specification/run/http2-protocol-core-client-promised-stream-id-ordering-human/`
  and
  `examples/specification/run/http2-protocol-core-client-promised-stream-id-ordering-json/`.
- The server-side outbound `PUSH_PROMISE` connection state separately retains
  the greatest successfully reserved local promised stream id. The first and
  increasing server-initiated ids advance that value only after the complete
  send intent has produced accepted frame bytes; repeated or lower ids use
  `http2.protocol.peer_stream_id_not_increasing` without exposing those bytes
  or committing HPACK, peer-settings, shutdown, associated-stream,
  promised-stream lifecycle, receive-credit, or ordering state. Domain,
  associated-stream lifecycle, peer `SETTINGS_ENABLE_PUSH`, GOAWAY, HPACK,
  frame-size, and generated encoding failures keep their focused paths. A
  higher id rejected by one of those validations remains eligible for a
  corrected retry, and accepted split `PUSH_PROMISE`/CONTINUATION output
  advances the retained id once. The broad executable evidence is
  `examples/specification/run/http2-protocol-core/`; focused server human and
  JSON projections are under
  `examples/specification/run/http2-protocol-core-outbound-promised-stream-id-ordering-human/`
  and
  `examples/specification/run/http2-protocol-core-outbound-promised-stream-id-ordering-json/`.
- The client-side outbound HEADERS connection state accepts an idle nonzero
  client-initiated stream id as a new local stream and retains the greatest id
  whose complete send intent was accepted. First and increasing ids advance
  that value only after HPACK encoding, frame splitting, peer frame-size and
  concurrent-stream checks, GOAWAY admission, stream-id encoding, and output
  construction succeed. Reused or lower new ids use
  `http2.protocol.peer_stream_id_not_increasing` without exposing the encoded
  bytes or committing HPACK, peer-settings, shutdown, receive-credit,
  lifecycle, or ordering state. Closing or resetting the latest stream does
  not lower the retained value, while HEADERS on its currently open stream
  keeps the existing lifecycle path without a new-stream ordering check.
  Server endpoints cannot use this path to start regular streams. Rejected
  higher ids remain eligible for corrected retry, and accepted split
  HEADERS/CONTINUATION output advances the value once. The broad executable
  evidence is `examples/specification/run/http2-protocol-core/`; focused human
  and JSON projections are under
  `examples/specification/run/http2-protocol-core-outbound-local-stream-id-ordering-human/`
  and
  `examples/specification/run/http2-protocol-core-outbound-local-stream-id-ordering-json/`.
- The checked HTTP/2 protocol core rejects server-side outbound `PUSH_PROMISE`
  send-intents on open associated streams, outbound `PRIORITY` send-intents
  on open streams, and stream-level outbound `WINDOW_UPDATE` receive-credit
  intents on open streams above a received or locally sent GOAWAY last-stream
  boundary before HPACK fixture encoding, frame splitting, priority payload
  encoding, receive-credit updates, or output chunk emission. Boundary
  streams remain accepted; connection-level outbound `WINDOW_UPDATE` remains
  accepted after GOAWAY subject to the existing increment and receive-window
  checks. Missing-stream, closed-stream, reset-stream, disabled-push,
  stream-id-domain, promised-stream id, priority self-dependency, increment
  range, receive-window overflow, and HPACK fixture failures keep their
  narrower facts. The checked case is
  `examples/specification/run/http2-protocol-core/`.
- The same checked HTTP/2 protocol core accepts a repeated local outbound
  GOAWAY send-intent only when it preserves or narrows the locally recorded
  graceful-shutdown last-stream boundary. Accepted repeated GOAWAY
  send-intents emit the normal GOAWAY frame bytes and update the local
  shutdown state to the sent boundary. Receiving or sending GOAWAY with no
  active stream at or below the recorded last-stream boundary exposes
  `drained_shutdown`; receiving or sending GOAWAY while an already-admitted
  in-boundary stream remains active continues to expose `graceful_shutdown`
  until that stream reaches a terminal state through received HEADERS
  `END_STREAM`, DATA `END_STREAM`, trailer HEADERS, `RST_STREAM`, or a local
  outbound DATA or HEADERS `END_STREAM` send-intent. When a local GOAWAY
  narrows an already received boundary, drain completion uses that stricter
  boundary. A repeated local
  GOAWAY that
  would widen the recorded boundary is rejected with
  `http2.protocol.stream_after_goaway` before output bytes are emitted. Later
  local outbound HEADERS, DATA, `PRIORITY`, stream-level `WINDOW_UPDATE`, and
  server-side `PUSH_PROMISE` send-intents continue to use the recorded local
  GOAWAY boundary. Later inbound stream-creating HEADERS after drain
  completion also use `http2.protocol.stream_after_goaway`, preserving the
  recorded shutdown boundary. The checked receive case covers already-admitted
  streams both at and below the received boundary, and rejects a new
  above-boundary HEADERS frame while graceful shutdown still has an active
  in-boundary stream. The checked case is
  `examples/specification/run/http2-protocol-core/`.
- The same checked HTTP/2 protocol core keeps one shared outbound connection
  window and a list of independently tracked outbound stream states. DATA
  consumes only the shared connection credit and the selected stream's send
  window and optional `content-length` count. Peer connection-level
  `WINDOW_UPDATE` changes only shared credit, stream-level `WINDOW_UPDATE`
  changes only the matching stream, and `SETTINGS_INITIAL_WINDOW_SIZE` applies
  its delta to every tracked open stream while preserving unrelated lifecycle
  and body-accounting facts. A reduction may make individual stream credit
  negative; a later update can restore that stream independently. Zero,
  overflow, unknown-stream, closed-stream, and reset-stream updates remain
  rejected without changing credit. The executable case checks three
  simultaneous streams as a minimum evidence set rather than a representation
  limit: `examples/specification/run/http2-protocol-core/`.
- The same executable core represents connection window credit, stream window
  credit, configured initial window size, and received `WINDOW_UPDATE`
  increments as distinct ordinary values backed by `Int`. Their constructors
  keep connection credit and initial sizes within zero and the HTTP/2 31-bit
  maximum, require increments to be nonzero and within that maximum, and allow
  stream credit to become negative after an initial-window reduction. DATA
  blocking and debit, SETTINGS deltas, and connection- and stream-level credit
  refill pass through these domains while preserving the existing observable
  protocol failures. The checked case directly covers accepted and rejected
  constructors, negative stream credit and DATA blocking, successful refill,
  and connection and stream overflow rejection under
  `examples/specification/run/http2-protocol-core/`.
- The checked HTTP/2 protocol core records one pending empty SETTINGS ACK
  send intent after a valid non-ACK peer SETTINGS frame with payload items.
  Multiple peer SETTINGS frames received before consumption coalesce to that
  one pending ACK. Consuming the intent emits an empty SETTINGS frame with the
  ACK flag and clears the pending ACK state without mutating peer-advertised
  settings. A non-ACK peer SETTINGS frame whose payload length is not a
  multiple of the six-byte SETTINGS item width fails with
  `http2.protocol.invalid_payload_length` before peer-advertised settings are
  updated; structured and human diagnostics keep the primary message on the
  payload-length fact and carry the SETTINGS item-width rule provenance as
  context. The checked cases are
  `examples/specification/run/http2-protocol-core/`,
  `examples/specification/run/http2-protocol-core-settings-item-length-json/`,
  and
  `examples/specification/run/http2-protocol-core-settings-item-length-human/`.
- A valid non-ACK peer SETTINGS frame processes items in wire order. Repeated
  known identifiers replace the earlier peer-advertised value, so the last
  occurrence is active; unknown identifiers interleaved between repetitions
  remain ignored. Repeated `SETTINGS_INITIAL_WINDOW_SIZE` items apply each
  ordered delta to every tracked open outbound stream while preserving
  connection credit, content-length accounting, and closed or reset stream
  lifecycle. The frame is atomic: all items are validated before peer state or
  derived stream credit is committed, and a later invalid repetition reports
  its own item offset without applying an earlier item from that frame. The
  integrated evidence is `examples/specification/run/http2-protocol-core/`;
  the focused range diagnostic projections are under
  `examples/specification/run/http2-protocol-core-settings-value-json/` and
  `examples/specification/run/http2-protocol-core-settings-value-human/`.
- The receive transition is role-aware for peer SETTINGS. A client endpoint
  rejects a peer-sent `SETTINGS_ENABLE_PUSH` item at that six-byte item's
  offset before applying any peer-advertised setting or state derived from the
  frame. The typed `http2.protocol.settings_not_allowed_for_endpoint` failure
  carries the setting identifier and name, endpoint role, SETTINGS frame kind,
  active state, rule provenance, and a bounded item preview. A server endpoint
  continues to accept `SETTINGS_ENABLE_PUSH` values `0` and `1`; unknown
  identifiers, SETTINGS ACK behavior, and the existing value-range checks are
  unchanged. The integrated, JSON, and human cases are
  `examples/specification/run/http2-protocol-core/`,
  `examples/specification/run/http2-protocol-core-settings-enable-push-role-json/`,
  and
  `examples/specification/run/http2-protocol-core-settings-enable-push-role-human/`.
- The same checked HTTP/2 protocol core accepts a local outbound PING request
  send-intent only when the opaque payload is exactly eight bytes. An accepted
  intent returns one immutable output chunk containing a length-`8`, kind-`6`,
  flags-`0`, stream-`0` frame header followed by the unchanged payload. Short
  and long payloads return a typed
  `http2.protocol.invalid_payload_length` outbound rejection before frame
  encoding and emit no output chunks. Existing inbound PING validation,
  automatic ACK emission for non-ACK PING frames, and no-response handling for
  received PING ACK frames remain checked by
  `examples/specification/run/http2-protocol-core/`.
- The same checked HTTP/2 protocol core accepts received `PRIORITY` frames on
  nonzero client-initiated streams when the frame has the fixed five-byte
  payload and does not depend on itself. The decoded frame exposes dependency
  stream id, exclusive flag, and weight. For tracked open and
  half-closed-local streams, accepted `PRIORITY` updates the stream priority
  facts and later accepted `PRIORITY` frames replace those facts without
  changing receive-window, content-length, HPACK, or shutdown state. Idle
  stream `PRIORITY` frames are decoded without opening a peer-created request
  stream or increasing concurrent-stream receive count. A tracked
  half-closed-local stream remains half-closed-local and can still receive
  DATA after the priority update. The checked case is
  `examples/specification/run/http2-protocol-core/`.
- The same checked HTTP/2 protocol core stores each tracked inbound stream as
  one entry in a recursive list. The entry owns its stream id, receive window,
  priority and content-length facts, and lifecycle. Lookup, admission, flow
  control, priority, reset, end-of-stream, and GOAWAY drain decisions use that
  list. The checked case admits five concurrent peer-created streams, applies
  DATA, stream-level `WINDOW_UPDATE`, and PRIORITY changes independently,
  resets one stream, admits a later stream without changing unrelated entries,
  and rejects a sixth open stream with
  `http2.peer_limit.concurrent_streams_exceeded` carrying the attempted and
  allowed list-derived counts. The checked case is
  `examples/specification/run/http2-protocol-core/`.
- The same checked HTTP/2 protocol core emits local SETTINGS send-intents as
  one frame-header-plus-payload chunk whose payload preserves caller item
  order. Supported local batch items are
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`,
  `SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
  `SETTINGS_MAX_FRAME_SIZE`, and `SETTINGS_MAX_HEADER_LIST_SIZE`. Accepted
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`, and
  `SETTINGS_MAX_HEADER_LIST_SIZE` local values must fit the HTTP/2 four-byte
  unsigned SETTINGS value field. Accepted batches are recorded as one
  outstanding local batch; one valid peer SETTINGS ACK clears exactly the
  oldest outstanding batch and leaves later batches pending. The checked case
  also fixes the no-output `local_settings` range-diagnostic path for an
  invalid item inside a larger batch.
- The checked HTTP/2 adapter/core write boundary composes ordinary pure
  response actions with pure HTTP/2 outbound HEADERS and DATA send-intents,
  then writes only accepted core-produced chunks through `net::write_chunks`.
  Adapter code preserves the core chunk order, including DATA frame splitting
  from the outbound credit and frame-size path. A rejected later DATA action
  remains an ordinary protocol decision and records no transport write for
  that action. The checked case is
  `examples/specification/run/http2-adapter-core-write-boundary/`.

## Runtime Output

The checked HTTP/2 protocol core supports
`SETTINGS_ENABLE_CONNECT_PROTOCOL` (`0x8`) as negotiated sans-I/O state.
Values `0` and `1` are accepted and other values use the structured SETTINGS
range diagnostic. Only servers advertise the setting; endpoint-role rejection
emits no bytes and preserves connection state. Once a server has locally
advertised value `1`, completed HEADERS and final CONTINUATION request blocks
may use extended CONNECT with exactly one non-empty `:protocol` plus
`:scheme`, `:path`, and `:authority`. The same validation rejects `:protocol`
on non-CONNECT requests and rejects extended CONNECT before negotiation, while
ordinary CONNECT retains its existing shape.

- `veln run` entries project returned `Result`, `ByteChunk`,
  `List<ByteChunk>`, HTTP/2 protocol diagnostics, and runtime diagnostic
  values through the command output boundary described in
  [commands.md](commands.md), [json-output.md](json-output.md), and
  [test-json.md](test-json.md).
- Executable specification cases may declare named binary fixture records and
  named output chunks in `case.toml`. These fixture records are harness
  expectations, not language syntax. A binary fixture may use `schema` to
  name a schema or schema alias from the fixture source module. Bare names may
  name private local schemas; qualified names require a written `use` path and
  a public target. Functions, source ADT types, codecs, generated helpers, and
  unresolved or inaccessible targets are rejected. When `field_path` is also
  present, its first segment must name the resolved schema. The checked
  positive and negative cases live under
  `examples/specification/run/binary-fixture-schema-references/` and
  `examples/specification/run/binary-fixture-schema-reference-diagnostics/`.
- Tail-recursive user functions may execute deep self-recursive calls through
  the implemented trampoline path. Other JVM details are backend details unless
  this reference marks a behavior as observable.

## Read When

- Core, typed IR, selected-entry reachability, and stdio ordering:
  [commands.md](commands.md).
- JVM lowering support, runtime containers, file-system, network, time, and
  binary helper details: [execution-full.md](execution-full.md).
- Machine-readable command output: [json-output.md](json-output.md) and
  [test-json.md](test-json.md).

## Skip Unless Needed

- Use [commands.md](commands.md) first for command gates and user-facing
  command behavior.
- Use checked examples before expanding this routing page.
