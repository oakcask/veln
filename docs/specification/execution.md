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
  work outside pure protocol code. Accepted and connected `NetStream` handles
  expose local and peer endpoint text through `net::stream_local_addr` and
  `net::stream_peer_addr` without exposing host socket handles or changing
  stream ownership. Production-loopback cases can preserve more than one
  configured read chunk for one accepted stream, and `net::connect` can return
  a deterministic client-side loopback stream with the same read, write, and
  close lifecycle; each chunk is observed by source as a separate read result
  before clean end. A source-visible production listen/connect lifecycle can
  also use one address value for `net::listen` and `net::connect`, accept the
  paired server stream, exchange a byte chunk across the two owned stream
  handles, close both handles, and then observe clean listener end before
  closing the listener. The
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
  That helper owns the `concurrency` boundary and calls a pure event/action
  handler. The
  multi-cycle routing case accepts more than one production stream from one
  listener and preserves repeated read, route, ordered write, close, and clean
  listener-end observations without exposing socket handles to handlers.
  Forced production read failure on the multi-chunk routing path stops after
  production accept and before later routing, response writes, stream close,
  or clean listener end.

## Binary Schemas

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
- Explicit schema encode expressions lower to the generated encode boundary
  for the referenced eligible binary schema. They typecheck the supplied value
  against the schema-local visible record shape and return
  `Result<ByteChunk, EncodeError>`. Public schema aliases lower through the
  same generated boundary as the aliased schema.
- Compatibility generated binary schema decode helpers read fields in
  declaration order and return the schema-local visible record shape. They are
  retained for old fixtures and runtime adapter coverage, not as the public
  source surface for applying schemas.
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
  `Result<T, String>` when every field is a scalar leaf, `Option<scalar>`,
  `Option<List<scalar>>`, `List<scalar>`, `Dict<String, scalar>`,
  `Result<scalar, scalar>`, or an anonymous record whose fields are supported
  format-neutral encode shapes. The supported scalar leaves are `Int`, `Bool`,
  `Float`, and `String`. The helper returns the supplied record on success
  and does not produce binary bytes.
- Repeated fields written as `[Payload; count]` normalize to the same generated
  decode and encode helper behavior as `Repeat(count, Payload)`, with the
  payload before `;` and the count expression after it. The count expression
  uses the same earlier-field and arithmetic forms accepted by `Repeat`.
  Lowercase exact-width `uint...` payloads written in legacy
  `Repeat(count, Payload)` fields normalize to the same generated decode and
  encode helper behavior as the matching canonical repeated-field payload.
  Repeated `ByteView(left_length - right_length)` payloads expose
  `List<ByteView>` and report truncation with the repeated element index.
- Direct nested binary schema fields name an eligible same-module or public
  imported nested binary schema, consume that nested schema in place, and
  expose the nested schema-local visible record at the field.
- Dispatch payload cases written with lowercase exact-width `uint...` and
  `flag...` primitive spelling normalize to the same generated decode and
  encode helper behavior as compatible upper-case exact-width payload spelling.
  Byte-aligned lowercase `uint... reserves <value>` dispatch payloads validate
  the fixed payload bytes during decode, emit those bytes during encode, and
  expose `()` as the payload value. Zero-reserved direct subbyte spellings
  from `uint1 reserves 0` through `uint7 reserves 0` consume one payload byte,
  validate the high-order payload bits, emit the same storage byte during
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
- Generated `validate_<schema>` helpers accept the schema-local decoded record
  shape and check field-local `where` predicates plus the single schema-level
  `validate` predicate when present.
- Generated binary schema encode helpers accept the schema-local visible
  record shape, validate field-local and representation constraints, and write
  bytes through the declared schema layout. Lowercase reserved-bit fields emit
  their declared values and are omitted from the input record like compatible
  `ReservedBits(width, value)` fields.
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
  fixture-marked values, accepting `http` and `https` and rejecting other
  visible ASCII values with `scheme_value_not_http_or_https` on completed
  HEADERS and final CONTINUATION paths. It also validates source-visible
  static-name `:authority` literal values through the existing request
  header-list path, accepting checked visible ASCII authority values and
  rejecting the checked invalid visible ASCII value with
  `authority_value_invalid` on completed HEADERS and final CONTINUATION paths.
  Stateful HTTP/2 response decoding validates `:status` pseudo-header values
  after fixture decode and after source-visible HPACK static-name literal
  decode. Accepted response lists keep exactly three ASCII decimal digits, and
  empty, short, long, or non-decimal values fail with
  `status_value_invalid` through the response header-list diagnostic on
  completed HEADERS and final CONTINUATION paths.
  Stateful HTTP/2 response decoding also accepts static-indexed
  `cache-control` and `content-type` entries after a static-indexed `:status`
  through completed HEADERS and final CONTINUATION paths in the same checked
  protocol-core case. Stateful HTTP/2 header-block decoding routes supported
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
  entry. It also decodes saturated seven-bit indexed representation
  `0xff 0x00` as HPACK index `127`, resolving dynamic table index `65` when
  the bounded carried table contains that retained entry. The boundary
  advances the dynamic-core decode count after each accepted decode and
  reports `hpack.fixture.dynamic_index_out_of_range` with requested dynamic
  index and entry-count facts when an indexed field asks past the carried
  table without advancing state. The same ordinary-source boundary exposes
  HPACK dynamic entry size as header-name byte count plus header-value byte
  count plus `32`, preserves immutable state while inserting newest-first
  dynamic entries, evicts oldest entries after insertion or table-size
  reduction including reduction to a zero-size table, and clears the table when
  an inserted entry is larger than the supplied table-size limit. The same
  boundary accepts the checked static-name literal-with-indexing block
  `content-type: text`, returns the decoded header entry and wire size, inserts
  it into the immutable dynamic-core state using the same accounting rule, and
  resolves a following `0xbe` dynamic indexed field from the inserted entry.
  It also accepts checked raw visible-ASCII literal-name fields across the
  literal-without-indexing, literal-with-indexing, and literal-never-indexed
  forms. Only literal-with-indexing mutates the immutable dynamic table; the
  other two forms advance the decode count without adding an entry. The
  checked raw literal-name insertion is then reused through the existing
  dynamic indexed path. HTTP/2 completed HEADERS and final CONTINUATION
  decoding route accepted raw literal-name fields through this source-visible
  boundary before fixture fallback, while unsupported or malformed forms keep
  the existing fixture fallback diagnostics. The checked cases are
  `examples/specification/run/hpack-fixture-codec-boundary/` and
  `examples/specification/run/http2-protocol-core/`.
- The source-visible HPACK fixture encoder accepts the checked outbound
  dynamic-name literal-with-indexing slice under
  `examples/specification/run/hpack-fixture-codec-boundary/` and routes the
  returned encode state through outbound HEADERS and server-side
  `PUSH_PROMISE` framing in
  `examples/specification/run/http2-protocol-core/`. The same fixture encoder
  observes the current outbound dynamic-table capacity after a checked
  table-size update: a later literal-with-indexing entry larger than the
  capacity is not retained for dynamic-index reuse, while an entry that fits
  the reduced capacity is retained and reused through the outbound HEADERS
  path.
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
- The checked HTTP/2 protocol core records one pending empty SETTINGS ACK
  send intent after a valid non-ACK peer SETTINGS frame with payload items.
  Multiple peer SETTINGS frames received before consumption coalesce to that
  one pending ACK. Consuming the intent emits an empty SETTINGS frame with the
  ACK flag and clears the pending ACK state without mutating peer-advertised
  settings. The checked case is
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
- The same checked HTTP/2 protocol core tracks a bounded multi-stream receive
  state for at least three concurrent peer-created open streams under the
  active receive limit. The checked case admits streams 1, 3, and 5, records
  their separate receive windows and priority facts, keeps DATA on one stream
  and stream-level `WINDOW_UPDATE` on another stream from mutating the third,
  and rejects the next peer-created stream with
  `http2.peer_limit.concurrent_streams_exceeded` carrying the attempted and
  allowed counts. The checked case is
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

- `veln run` entries project returned `Result`, `ByteChunk`,
  `List<ByteChunk>`, HTTP/2 protocol diagnostics, and runtime diagnostic
  values through the command output boundary described in
  [commands.md](commands.md), [json-output.md](json-output.md), and
  [test-json.md](test-json.md).
- Executable specification cases may declare named binary fixture records and
  named output chunks in `case.toml`. These fixture records are harness
  expectations, not language syntax.
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
