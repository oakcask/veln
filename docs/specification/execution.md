---
role: specification
authority: normative
specification-coverage: usage=#runtime-readiness-and-host-boundaries; behavior=#runtime-readiness-and-host-boundaries; limits=#observable-output
update-when: The checked-core readiness, typed-IR readiness, runtime execution, codec, JVM backend, or execution evidence contract changes.
---

# Execution Boundary

This page is the authority for execution readiness, runtime values, codecs,
host boundaries, and observable output. HTTP/2 protocol details belong to
[http2.md](http2.md).

## Runtime readiness and host boundaries

Semantic diagnostics must be error-free before the compiler produces checked
core and typed IR. Command analysis checks readiness for the selected entry
before applying command-specific execution or write policy. Holes, missing
expressions, constructor or call arity failures, and recognized concurrency
blockers prevent execution.

The JVM backend emits classfile artifacts and invokes the selected entry. Java
source generation and Java source compilation are outside the command contract.
`ByteChunk`, `ByteView`, `StreamInput`, `DecodeStep<T>`, `DecodeReadiness`,
`DecodeError`, `EncodeStep<TState>`, and `EncodeError` are immutable
source-visible values. Byte and view helpers are pure; invalid values,
malformed compact hex, out-of-range counts or ranges, truncated fixed-width
reads, schema mismatches, and conversion overflow return typed failures while
preserving their inputs.

`byte_chunks_produce(chunks, budget)` emits only whole chunks that fit the
`ByteCount` budget, preserves order, reports the produced count, and returns
the unproduced suffix. It does not mutate the input list or chunks.

`net` and `time` are host effects. Socket handles, deadlines, cancellation,
and monotonic clocks remain in the host runtime. `NetListener` exposes local
endpoint text through `net::listener_local_addr`; `NetStream` exposes local and
peer endpoint text through `net::stream_local_addr` and
`net::stream_peer_addr`. These queries do not transfer host handles, close
resources, or change ownership.

A listener accepts streams until clean listener end or a host failure.
`net::read_chunk_or_end` returns a chunk or clean end. The synthetic transport
returns configured chunks separately; host socket reads follow the bytes
available from the socket and do not preserve peer write boundaries. A write
preserves list order. Stream close and listener close are explicit ownership
operations. Read shutdown preserves write ownership and makes later optional
reads clean end; write shutdown makes later writes transport failures. A
closed handle remains invalid, and operations through it return transport
failures without reopening the resource. Loopback and external transports use
the same source-visible result shapes; external transport binds and connects
real host sockets and has no synthetic fallback.

Host listen, connect, accept, read, write, shutdown, stream-close, and
listener-close failures expose one structured transport payload. It retains
operation and category, known endpoints and owned identity, lifecycle phase,
input/output/ownership commit facts, and the related platform cause. Failures
before a commit, after input consumption, after output commit, after stream
close, and through a stale handle remain distinguishable.

The cancellable receiver-list adapter observes routed, timed-out, and
cancelled outcomes before producing adapter actions. It uses channel-routed
`StreamInput` values and preserves adapter trace identity and event order.
`stream_adapter_drain_actions` drains one accepted stream, emits only
`SendBytes` actions through `net::write_chunks`, and leaves stream and listener
closure to its caller. `stream_adapter_accept_loop` accepts, drains, writes,
closes each stream once, and closes the listener after clean end.
`stream_adapter_drain_actions_until_cancellable` uses the supplied `Deadline`
and `CancelToken`; deadline and cancellation are ordinary outcome values,
while host writes remain transport failures. A forced read failure stops the
cycle before later routing or cleanup actions.

For a concurrent stream service that drains in acceptance order, retain each
accepted `NetStream` with its `Task<Result<HandlerOutput, String>>`, then join
that list in order and close each owned stream. To stop after a handler or join
failure, the service must explicitly cancel and join pending tasks and close
their streams. This orchestration belongs to the source program; task failure
does not automatically close its host resources or undo completed writes.

`transport::net::net_stream(stream)` adapts a caller-owned `NetStream` to the
`DuplexStream` interface. `read_chunk()` returns `Some(ByteChunk)` or `None`
for clean end; `write_chunks(chunks)` writes in list order. The adapter does
not listen, connect, accept, close, or shut down the stream.

HTTP/2 connection drivers own only the core state and caller-owned duplex
interface. The client writes its preface and initial SETTINGS before reading;
the server writes its initial SETTINGS before reading. Closed entry state
returns without I/O. Protocol failures are typed core failures, while host
transport failures remain runtime failures. Detailed HTTP/2 frame, HPACK,
connection-state, and service contracts are specified in [http2.md](http2.md).

`serve_connection` maps one callback boundary to the HTTP/2 driver and
preserves the callback effect row. `serve_tcp` owns the listener, accepted
streams, and connection tasks, closes each owned stream once, closes the
listener on clean end or first ordinary service failure, and stops accepting
after the first application or join failure. `request_endpoint_sequence`
owns each client stream and task, reuses a connection only while the core
state is open and peer stream capacity remains, otherwise closes and
reconnects. Callback, protocol, and join failures are typed service failures;
abrupt transport failures remain runtime failures.

## Binary schemas

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
  path corresponding to the missing input position.
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
  schema names, and failing nested field path.
- Schema composition fields consume or pass through the resolved target schema
  in place and expose its visible record only beneath the containing field
  binding. Format-neutral targets preserve their nested record through decode
  and encode; binary targets consume and emit their nested representation in
  field order. Later expressions resolve explicit paths into earlier composed
  records. A format-neutral composed target runs its field predicates and
  schema validation before the containing schema continues validation. Nested
  decode and encode failures retain the containing schema and binding before
  the target schema path, and neither direction commits a partial containing
  value or byte sequence. A failed nested attempt does not affect a later
  successful call.
- Anonymous record fields in `format binary` schemas consume their exact-width
  unsigned primitive leaves in source order and expose the nested anonymous
  record shape at the field. Anonymous records may recurse when all leaves are
  exact-width unsigned primitives, and a record may contain sibling nested
  anonymous record fields at the same level. Truncation inside the anonymous
  record preserves the outer field segment and appends every anonymous record
  field segment down to the failed primitive.
- Generated binary schema encode helpers accept anonymous record fields whose
  leaves are exact-width unsigned primitives, write leaves in declaration
  order with each primitive's byte order, and preserve nested anonymous record
  field segments in `schema.encode_value_unrepresentable` paths.
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
  nested failures preserve their parent path, selected schema, element index,
  and nested field path.
- Same-module recursive dispatch payload cases expose a finite primitive
  payload shape when the recursive dispatch field is length-bounded and has a
  non-recursive primitive base case. Decode helpers collapse recursive known
  payload chains to that primitive payload value. Encode helpers accept the
  same schema-local visible shape and can encode the primitive base case while
  preserving the usual dispatch length checks. A missing non-recursive base
  is rejected before a recursive helper is generated.
- Representation-only fields such as supported `ReservedBits(width, value)`
  and lowercase `uint... reserves <value>` layouts are validated and omitted
  from the decoded record.
- A direct `ReservedBits(width, value)` followed by `UInt8` uses one
  big-endian storage group when the width is positive and non-byte-aligned,
  the value fits the width, and the group with trailing padding fits in at
  most eight bytes. Decode and encode omit the reserved field, expose the
  visible byte, consume the full padded group, and report truncation or
  `schema.reserved_bits_mismatch` at the declared field paths. Boundary
  failures retain their declared field paths.
- Generated `validate_<schema>` helpers accept the schema-local decoded record
  shape and check field-local `where` predicates plus the single schema-level
  `validate` predicate when present.
- Generated binary schema encode helpers accept the schema-local visible
  record shape, validate field-local and representation constraints, and write
  bytes through the declared schema layout. Lowercase reserved-bit fields emit
  their declared values and are omitted from the input record like compatible
  `ReservedBits(width, value)` fields.
- Direct visible big-endian `UInt16be`, `UInt24be`, `UInt31be`, and
  `UInt32be` helpers decode and encode their declared widths. Five-byte
  `UInt40be` and `UInt40le`, six-byte `UInt48be` and `UInt48le`, seven-byte
  `UInt56be` and `UInt56le`, and eight-byte `UInt64be` and `UInt64le` helpers
  do the same in their declared byte order. The source-visible
  `byte_read_u56_be`, `byte_read_u56_le`, `byte_write_u56_be`, and
  `byte_write_u56_le` helpers expose the corresponding seven-byte operations.
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
  source at the caller or schema-operation boundary.
- Schema-level `map to` clauses are rejected by the parser before execution.
  Schema-level rejection is reported before execution.

## Codecs and protocol adapters

Source-level `codec` and `pub codec` declarations are rejected before
execution. Decode and encode entry points are ordinary functions or explicit
schema operations. Compatibility diagnostic identifiers under `codec.*` remain
available where compatibility values expose them; they do not add a new source
declaration form.

Byte decoders may return `codec.trailing_input` when a caller explicitly
requires full consumption. Decoders do not reject trailing bytes by default.
`DecodeErrorWithReason` and `DecodeStep::Invalid` preserve consumed,
available, and remaining byte counts when those counts are known. A rejected
decode or encode returns no committed value or bytes; a `DecodeStep` readiness
result may instead request more input as part of its public partial-read
contract. Successful incremental encoding can return `EncodeStep::Partial`
with output chunks, a produced count, and continuation state; partial output
is not itself an encode failure.

The production HTTP/2 HPACK API is authoritative for prefixed integers,
Huffman strings, static and dynamic tables, table-size updates, indexed and
literal fields, and complete header blocks. Its immutable transitions and
typed failures are specified in [http2.md](http2.md). The execution boundary
does not duplicate those rules.

Compatibility fixture adapters expose these callable helpers: static indexed
and static-name literal forms, bounded raw or Huffman values, dynamic indexed
and dynamic-name literal forms, table-size updates at the start of a block,
and finite header-block encode/decode. Their dynamic index and update limits
are bounded by the carried fixture table and its configured peer maximum.
Unsupported representations return `hpack.fixture.*` failures without output
or table mutation. These limits apply only to the compatibility helpers;
production callers use `http2::hpack`.

Compatibility fixture encoders preserve immutable input state. They emit only
complete header-field or header-block bytes, select a static exact match before
a literal fallback, and return no bytes when a name, value, representation,
dynamic index, table capacity, or Huffman input is unsupported. A successful
incrementally indexed literal updates the returned compatibility table; other
literal policies preserve it. Fixture dynamic-index and table-size failures
report the requested index or capacity and leave the input state unchanged.

The shared HTTP/2 header-list validator applies after production or
compatibility decoding. Its request, response, trailer, CONNECT, extended
CONNECT, size, and content-length rules are authoritative in
[http2.md](http2.md). A failed validation preserves HPACK, stream, and output
state.

Production schema operations and compatibility adapters preserve caller state
on every failure. A nested codec failure keeps its parent field path and
reports the absolute byte offset or field position available at that boundary.
Representational failures remain distinct from transport and protocol
failures.

## Observable output

Execution writes only committed byte chunks. A failed encode, rejected core
decision, or uncommitted action produces no bytes for that action. A host write
failure can occur after the host commits output; its commit facts and earlier
chunks remain observable. Earlier committed chunks remain observable when a
later action fails.
Output buffers preserve append order, expose chunk count and indexed chunks,
and can produce the concatenated byte sequence without changing stored chunks.

Command-facing protocol failures retain their stable identifier, operation or
transition source, byte offset, frame kind and stream where known, active-state
label, rule provenance, and preview or related cause. Human rendering keeps the
primary failed fact at its span; structured context belongs in related details.
Runtime transport failures remain transport failures and are not converted to
protocol or application failures.

## Read When

- Use [http2.md](http2.md) for production HTTP/2 frame, HPACK, connection,
  header-list, and service behavior.
- Use the command-specific pages for CLI analysis, JSON records, and output
  envelopes.
- Use [examples.md](examples.md) to locate representative executable evidence.
- Use [source-decisions.md](source-decisions.md) only for rationale.

## Skip Unless Needed

- Do not use compatibility fixture adapter behavior as a production API
  guarantee.
- Do not use proposal pages or fixture manifests as current runtime rules.
