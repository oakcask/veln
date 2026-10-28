---
role: specification
authority: normative
specification-coverage: usage=#runtime-readiness-and-host-boundaries; behavior=#runtime-readiness-and-host-boundaries; limits=#cleanup-limits
update-when: The checked-core readiness, typed-IR readiness, runtime execution, source-visible runtime or binary-schema codec behavior, JVM backend, or execution evidence contract changes.
---

# Execution Boundary

This page is the authority for execution readiness, runtime values, codecs,
host boundaries, and observable output. HTTP/2 protocol details belong to
[http2.md](http2.md).

## Runtime readiness and host boundaries

Explicit import aliases affect source lookup only. Lowering and reachability
use the selected target module and declaration identity for calls,
constructors, schemas, effects, and handlers. An invalid, recovered, reserved,
or colliding alias cannot itself make a module executable or add declarations
to the reachable set. Within one importing owner package and source module,
imports that resolve to one target module in one package origin are duplicate
targets and add no reachable declarations, even when a standard module is
written once as a relative path and once with its canonical `std::` path.
Equal target spellings in different importing owner packages remain
independent. A valid exact written path remains usable when only its local
qualifier collides. An alias-qualified call and a call through the full written
import path execute the same declaration. Same-named declarations with
distinct package or module identities retain separate runtime identities, and
an explicitly aliased import does not add a bare runtime call candidate. An
import-selected function or handler keeps its selected package identity
through reachability, lowering, and JVM linkage; a loaded namesake from another
package cannot replace the selected declaration. An imported handler retains
the declaration module as the lookup scope for private functions called by its
operation clauses. The checked
[`explicit-import-alias-runtime`](../../examples/specification/run/explicit-import-alias-runtime/)
and
[`explicit-import-alias-handler`](../../examples/specification/run/explicit-import-alias-handler/)
cases exercise call and handler behavior through the JVM backend. The
[`explicit-import-alias-handler-identity`](../../examples/specification/run/explicit-import-alias-handler-identity/)
case checks that same-named handlers keep distinct declaration identities.
A run entry also retains its declaring-module identity when its reachable set
contains an imported function with the same name. The selected entry, rather
than the same-named imported declaration, is the JVM launch target. The checked
[`explicit-import-alias-entry-target`](../../examples/specification/run/explicit-import-alias-entry-target/)
case exercises that boundary.
A local application module whose written path starts with `std::` keeps
application runtime identity. Its full written path and an explicit alias can
execute the local declaration, while the standard-relative leaf alone remains
unresolved. The checked
[`explicit-import-alias-local-std-runtime`](../../examples/specification/run/explicit-import-alias-local-std-runtime/)
and
[`explicit-import-alias-local-std-origin`](../../examples/specification/run/explicit-import-alias-local-std-origin/)
cases cover the successful JVM linkage and the rejected relative lookup.
Import selection and quarantine also retain the package that owns the importing
source. Application and standard-library sources with the same module spelling
therefore keep independent import bindings through reachable lowering. The
checked
[`explicit-import-alias-owner-package-scope`](../../examples/specification/run/explicit-import-alias-owner-package-scope/)
case executes a standard import while a loaded same-spelled application source
owns a separate import.

The JVM host boundary associates resources created by `net::system()` with the
active `net::IO` handler frame. Popping that frame closes its remaining
listeners and streams before restoring the outer handler. The cleanup runs for
normal completion, propagated errors, and runtime exceptions. A copied handler
stack inherited by a child task refers to the same owner, so concurrent
operations observe the same close and ownership state. The frame becomes
closed to resource publication before cleanup starts. A child operation that
finishes creating a listener or stream after that transition cleans the new
resource according to its reported commit state and returns `InvalidResource`
instead of publishing it through the closed frame.
The frame closes retained resources in reverse publication order. A stream
accepted from a retained listener is therefore closed before that listener.
For the JVM lifecycle workload of distinct retained streams whose close
operations succeed, resource-release lookup work scales linearly with the
number of retained streams. The deterministic lifecycle harness counts
resource-identity equality checks instead of elapsed time. It compares 4,000
and 8,000 retained streams, permits at most four checks per stream, and requires
the larger workload to use no more than three times the smaller workload's
checks.
The [standard-library networking specification](standard-library-networking.md#network-operation-boundary)
owns the resolver controls, connection commit point, exact write-progress,
concurrent-operation, and resource state transitions for this handler. Task
cancellation stops the waiting system-handler call before join reports
cancellation. A platform resolver that ignores interruption can remain active
within the handler's finite resolver capacity.

The JVM runtime also provides the bounded metric-series index reached by the
embedded standard package through private `production_metric_index_*`
compiler adapters. Application source cannot resolve those adapters as bare
prelude names or public standard-package declarations. The
[standard-library observability specification](standard-library-observability.md#production-metric-cardinality)
owns the source-visible series identity, admission, health, cancellation, and
state-bound behavior.

The JVM runtime provides the JSON record encoder, configured destination
writer, and identity generator reached by the JSON Lines observation handler.
Without a scoped `host_effects::ObservationExporter` handler, the writer
flushes standard-error records or creates and appends UTF-8 bytes to the
configured file, and the identity generator adds a host-generated UUID to the
configured prefix and identity kind. A scoped Veln handler can replace writes
and identities for deterministic tests. The
[standard-library observability specification](standard-library-observability.md#synchronous-json-lines-export)
owns the source-visible schema, sequencing, limits, failure health, and
destination choices.

If network-handler cleanup cannot confirm that every retained resource is
closed, leaving the handled scope produces a cleanup failure after attempting
the owner's other resources. That failure becomes primary after a successful
body. An `Err` selected for propagation by postfix `?` is still an ordinary
return value at this boundary. If handler cleanup fails while that return is
unwinding, the cleanup failure replaces the pending `Err`, and enclosing
deferred blocks and handler frames continue to unwind. If a contract failure,
runtime failure, or cancellation is already primary, it remains primary and
the network-handler cleanup failure is attached as a related cleanup failure
while unwinding continues.

Semantic diagnostics must be error-free before the compiler produces checked
core and typed IR. Command analysis checks readiness for the selected entry
before applying command-specific execution or write policy. Holes, missing
expressions, constructor or call arity failures, and recognized concurrency
blockers prevent execution.

Module immutable value declarations do not reach checked core or typed IR.
Shared semantic analysis checks their written annotations, finite-data
initializers, eligible imported references, package-aware visibility, and
same-module dependencies. It then emits
`module.value_not_executable` for every declaration, which blocks execution
before code generation. Parsing, formatting, and semantic checking for the
declaration do not create a runtime value or initialization phase.

A variant-refinement visibility failure anywhere in a public function
parameter or result, or in a public variant payload of a public source ADT, is
also a checked-core and typed-IR blocker. This applies when the written base
names the terminal ADT directly or resolves to it through a finite transparent
type-alias chain. The
failed function publishes no callable signature, and the failed ADT publishes
no imported public type fact. The same blockers close over ordinary named ADT
dependencies in public payloads, including transparent aliases and recursive
dependency cycles. A public function is also blocked when a parameter or
result contains an ADT in that closure. These ordinary references do not
produce a second privacy diagnostic. A direct or chained function alias cannot
restore execution readiness for an unpublished function. The
[type-checking contract](types.md#compatibility-and-limits) defines the
affected annotation positions and the valid private-function controls.

A refinement union whose successfully instantiated alternatives have different
resolved ADT identities or canonical generic arguments is also a checked-core
and typed-IR blocker. The invalid annotation quarantines the complete
containing function, effect operation, handler, or ADT-constructor signature,
so lowering cannot recover execution readiness from a valid sibling type. The
type-checking contract defines the comparison and quarantine boundary, and the
[diagnostic contract](diagnostics-json.md#diagnostic-families) defines the
focused failure and independently retained diagnostics.

Direct and indirect calls to call-site-aware functions enter checked core with
a hidden `SourceLocation` argument. Calls from ordinary functions construct
that value from the call expression's complete resolved or generated span,
while call-site-aware wrappers forward their existing value. A generated span
resolves to its original path and coordinates only when both boundaries map;
otherwise, execution keeps the complete generated span. Runtime-backed callback
consumers bind the caller's context to a call-site-aware callback, including a
callback that runs in an asynchronous task. A runtime contract on a
call-site-aware function forwards its existing value when it directly calls
another call-site-aware function.
Library-generated spans use a canonical virtual path derived from the
package-relative source path and generator identity. The exposed path is
unchanged when the source tree moves between absolute roots. A captured
`SourceLocation` is an ordinary value: returning or storing it preserves the
same location for later observation after the originating function returns.
Call-site-aware run entries and the same contract call from an ordinary
function are execution gates. The ordinary-function contract deliberately does
not construct hidden call-site context. Direct built-in references in runtime
contracts execute with the function's supplied value.
The focused declaration and propagation contract is specified by
[Call-site Declarations](call-site-declarations.md).

`begin` expressions and `defer` statements pass parsing, static analysis,
checked-core readiness, and typed-IR readiness. Public run and test commands
execute reachable programs containing either form.

Checked core and typed IR represent cleanup
regions, deferred blocks, and registration-time snapshots of referenced local
bindings. The JVM backend executes blocks that were reached and registered
exactly once in reverse registration order when a region completes normally,
postfix `?` propagates an `Err`, or a contract or runtime failure throws from
the region body. Abrupt exits unwind an inner region before its enclosing
region and do not run blocks after the failing expression. Every path uses the
registration-time snapshots. Cleanup registrations and effect-handler frames
unwind in reverse lexical nesting order.

Every registered cleanup runs even when an earlier cleanup fails. If the
region was successful, the first cleanup failure becomes primary after all
cleanup has run. If a contract failure, runtime failure, or cancellation was
already primary, it remains primary and cleanup failures are attached in
execution order as related failures. Human execution prints those failures as ordered related
cleanup lines; run JSON exposes the same order in `error.related`. Task
cancellation waits for the task worker to leave its cleanup regions before
join reports cancellation. The effect boundary during unwinding is specified by
[effects.md](effects.md#effect-labels). The backend transfers a successful
`begin` value only after that region's cleanup completes.

Integer remainder evaluates the left operand once and then the right operand
once. A failure in the left operand prevents evaluation of the right operand.
A zero divisor discovered at runtime raises `runtime.remainder_by_zero` and
stops execution before later expressions while registered cleanup unwinds.
Human output reports `remainder divisor is zero` at the divisor span and
identifies the complete operation in a related note. Earlier output remains
observable. The [operator specification](types.md#operators-and-diagnostics)
defines the signed result, and [run JSON](run-json.md) defines the structured
failure projection. The same runtime behavior applies when `%` is evaluated
inside a runtime-required `require`, `ensure`, or `invariant` predicate. A
literal zero divisor is rejected during analysis instead, as specified by
[contracts](contracts.md#predicate-usage-and-validation).

## Cleanup limits

Registered cleanup is guaranteed only while the Veln task, JVM, and executing
machine continue to run. Host-process termination, virtual-machine
termination, or loss of the executing machine can prevent cleanup from
running. Cleanup is lexical execution, not garbage-collection finalization.
Cleanup does not recover or replace an existing contract or runtime failure.

`defer` does not implicitly close or flush a value. A deferred block must call
the required operation explicitly and handle a meaningful close or flush
result explicitly. The language does not define a disposable
interface, destructor protocol, or automatic resource-management convention.
`begin` defines a cleanup region; it does not add rescue, ensure, or exception
syntax.

The JVM backend emits classfile artifacts and invokes the selected entry. Java
source generation and Java source compilation are outside the command contract.
`ByteChunk`, `ByteView`, `StreamInput`, `DecodeStep<T>`, `DecodeReadiness`,
`DecodeError`, `EncodeStep<TState>`, and `EncodeError` are immutable
source-visible values. Byte and view helpers are pure; invalid values,
malformed compact hex, out-of-range counts or ranges, truncated fixed-width
reads, schema mismatches, and conversion overflow return typed failures while
preserving their inputs.

On the JVM, `byte_take`, `byte_drop`, and `byte_view_to_chunk` can share an
immutable backing range. Repeatedly dropping one-byte prefixes performs work
linear in the original chunk length. A non-empty derived chunk retains backing
capacity no greater than four times its logical byte count, and an empty
derived chunk retains no backing elements. The deterministic byte-chunk slice
harness counts created ranges and copied elements and inspects retained
capacity; it does not use wall-clock timing.

`byte_chunks_produce(chunks, budget)` emits only whole chunks that fit the
`ByteCount` budget, preserves order, reports the produced count, and returns
the unproduced suffix. It does not mutate the input list or chunks.

`net` and `time` are host effects. Socket handles, deadlines, cancellation,
monotonic clocks, and wall-clock acquisition remain in the host runtime. A
wall-clock read crosses that boundary as one normalized `WallTime` record with
integer `unix_seconds` and `nanosecond` fields. The default reads the real host
wall clock; a scoped Veln clock handler can replace it deterministically. Wall
time can move backwards and is not used for deadline ordering. The
normalization guarantee belongs to the boundary result, not to independently
constructed records that have the same structural fields. A handler result
whose normalization would exceed the `Int` seconds range is a runtime failure.
The exported `std::net` module exposes opaque `net::Listener` and `net::Stream`
aliases for the host resources. `net::listener_address` returns a typed
`net::Endpoint` for a listener. `net::local_address` and `net::peer_address`
return typed endpoints for a stream. These queries do not transfer host
handles, close resources, or change ownership.

A listener accepts streams until clean listener end or a host failure.
`net::read` returns a typed `net::ReadOutcome` or `net::NetError`. Injected
network handlers can return separate chunks; default host socket reads follow
the bytes available from the socket and do not preserve peer write boundaries.
`net::write_all` preserves the bytes in one chunk and retries uncommitted
suffixes until completion or the first typed failure. Stream close and
listener close are explicit ownership operations. Read shutdown preserves
write ownership and makes blocked or later reads return `Closed`; write
shutdown makes blocked or later writes return `Closed`. A closed handle remains
invalid, and operations through it return typed failures without reopening the
resource. Loopback and remote
connections use the same source-visible result shapes. The default transport
binds and connects real host sockets. The complete source-level operation and
failure contract is specified by
[Standard-library Networking](standard-library-networking.md).

Host listen, connect, accept, read, write, shutdown, stream-close, and
listener-close failures expose one structured transport payload. It retains
operation and category, known endpoints and owned identity, lifecycle phase,
input/output/ownership commit facts, and the related platform cause. Failures
before a commit, after input consumption, after output commit, after stream
close, and through a stale handle remain distinguishable.

The cancellable receiver-list adapter observes routed, timed-out, and
cancelled outcomes before producing adapter actions. It uses channel-routed
`StreamInput` values and preserves adapter trace identity and event order.
`stream_adapter_drain_actions` drains one accepted stream and writes only
`SendBytes` payloads in list order through its private host-network
adapter, and leaves stream and listener closure to its caller.
`stream_adapter_accept_loop` accepts, drains, writes,
closes each stream once, and closes the listener after clean end.
`stream_adapter_drain_actions_until_cancellable` uses the supplied `Deadline`
and `CancelToken`; deadline and cancellation are ordinary outcome values,
while host writes remain transport failures. A read failure stops the
cycle before later routing or cleanup actions.

For a concurrent stream service that drains in acceptance order, retain each
accepted `NetStream` with its `Task<Result<HandlerOutput, String>>`, then join
that list in order and close each owned stream. To stop after a handler or join
failure, the service must explicitly cancel and join pending tasks and close
their streams. This orchestration belongs to the source program; task failure
or cancellation does not automatically close caller-owned listeners,
streams, standard input, or undo completed writes.

`transport::net::net_stream(stream)` adapts a caller-owned `net::Stream` to the
`DuplexStream` interface and retains `net::IO`. `read_chunk()` returns
`Ok(Some(ByteChunk))` for non-empty data, `Ok(None)` for clean end, and the
unchanged typed network error for failure. A successful empty read is a typed
non-progress error. `write_chunks(chunks)` completes each chunk in list order
through `net::write_all` and returns the first typed failure. The adapter does
not listen, connect, accept, close, or shut down the stream.

HTTP/2 connection drivers own only the core state and caller-owned duplex
interface. The client writes its preface and initial SETTINGS before reading;
the server writes its initial SETTINGS before reading. Closed entry state
returns without I/O. Protocol failures are typed core failures. Duplex read
and write failures are distinct typed transport failures at the direct
connection and application boundaries. Detailed HTTP/2 frame, HPACK,
connection-state, and service contracts are specified in [http2.md](http2.md).

`serve_connection` maps one callback boundary to the HTTP/2 driver and
preserves the callback effect row. `serve_tcp` owns the listener, accepted
streams, and connection tasks, closes each owned stream once, closes the
listener after the first failure, and stops accepting after the first network,
application, or join failure. The TCP service uses the standard network duplex
adapter under the caller's `net::IO` handler. Its typed network failure retains
the accepted count, and a later cleanup failure replaces the earlier outcome.
`request_endpoint_sequence`
owns each client stream and task, reuses a connection only while the core
state is open and peer stream capacity remains, otherwise closes and
reconnects. Callback, protocol, and join failures are typed service failures;
connect, stream I/O, and stream-cleanup failures are typed network failures
that retain the originating `NetError` and completed-request count. A cleanup
failure replaces an earlier client outcome without changing that count. The
client and its spawned tasks use the caller's `net::IO` handler.
`request_endpoint_sequence_tcp` preserves that service contract and installs
`net::system()` for its client scope. Its callers supply host `net`, `time`,
and `concurrency` effects instead of a `net::IO` handler. Direct protocol
drivers, the duplex adapter, the lower client service, and `serve_tcp` do not
install that production client boundary.

## Schema execution

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
- Explicit schema validation evaluates its erased base-record value once and
  then its `ByteOffset` once. For supported direct refinement fields, runtime
  checks follow schema field declaration order and accept constructors in the
  refinement's source-ADT declaration order. Success returns the identical
  record value in `Ok` with the refined static type. The first excluded
  constructor returns `Err(DecodeErrorWithReason(...))` with
  `schema.variant_refinement_mismatch`, the exact supplied offset, and a JSON
  Pointer formed from the terminal schema declaration and field names. Pointer
  tokens escape `~` as `~0` and `/` as `~1`; selecting a public alias does not
  replace the terminal declaration root. A negative supplied offset fails
  before field traversal with `schema.validation_offset_invalid` at the schema
  root. Validation performs no byte I/O and does not invoke a schema decoder.
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
  shape through the pass-through boundary. An eligible singleton or
  finite-set refinement remains in both the helper parameter and successful
  `Result` type through supported containers, anonymous records, and composed
  format-neutral schemas. The supplied record must be assignable to that
  refined parameter. A base ADT or an excluded constructor at a refined
  position fails static checking with `type.mismatch`.
  A schema that declares the base ADT retains the base type in its successful
  result; the constructor observed at runtime does not strengthen that result.
  On success the helper returns the same value. It performs no runtime
  refinement check and introduces no wire representation. Helper visibility
  follows the [schema-operation rules](source-surface.md#schema-operations):
  an exact test companion may call an explicitly imported private target
  helper whose signature contains a private refinement, but a public helper
  signature cannot expose a private constructor and ordinary importers gain no
  private access.
- A representation decoder can produce a refined ADT only when it directly
  constructs an admitted variant or can inspect and validate the decoded ADT
  tag. A `format binary` schema selects a generated representation decoder.
  That decoder cannot establish a finite ADT refinement written in one of its
  fields, so analysis rejects the innermost such resolved target with
  `schema.variant_refinement_decode_unsupported` before execution. Merely
  observing a constructor tag does not strengthen a decoder result declared as
  the base ADT. Format-neutral generated decode helpers are the supported
  contrast: their refined parameter statically requires the caller to supply
  an admitted value, and the typed pass-through result retains that refinement
  without inspecting a runtime tag.
- Format-neutral generated encode helpers for schemas without a `format`
  clause accept the schema-local record defined by the
  [format-neutral field eligibility rules](source-surface.md#format-neutral-fields)
  and return `Result<T, String>`. An eligible singleton or finite-set
  refinement of a source ADT remains refined in the helper's input and
  successful result through supported containers, anonymous records, and
  composed format-neutral schemas.
  The helper accepts an admitted constructor. A base ADT or an excluded
  constructor at any such refined position fails static checking with
  `type.variant_mismatch`, including when the refined position is supplied by
  a list or vector literal, dictionary literal, or `Some`, `Ok`, or `Err`
  payload inside the record. An unrelated nested type difference remains
  `type.mismatch`. Assigning a generated encode helper to an immutable local
  binding preserves this schema-boundary checking when the binding is called;
  assigning an ordinary function with the same visible parameter shape does
  not acquire it.
  The helper returns the supplied record on success and does not produce binary
  bytes, so an admitted refined value has the same representation as the
  corresponding base-ADT encode path. Checked runtime and diagnostic evidence
  covers singleton, union, nested-container, and pass-through behavior, plus
  direct, nested-container, and composed-schema rejection of base and
  excluded-constructor values. Helper visibility follows the
  [schema-operation rules](source-surface.md#schema-operations): an exact test
  companion may encode through its explicitly imported private target schema,
  including a schema-local private refinement, without making that helper
  available to ordinary importers or through a public schema boundary.
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
protocol or application failures. When `net::system()` converts a failed host
`listen` or `connect` into a typed `NetError`, the JVM carries the transport
facts for that pre-commit attempt with the value. If that error becomes the
selected entry result, command reporting observes the transport failure rather
than degrading it to a generic rendered result value.

## References

- Cross-package function and handler selection through reachability, lowering,
  and JVM linkage, together with validation operand evaluation order, are
  checked in `crates/veln-backend-jvm/src/tests/basic_backend.rs`.
- Direct-field runtime refinement success and declaration-order failure are
  checked by
  [`schema-runtime-refinement-validation`](../../examples/specification/run/schema-runtime-refinement-validation/).
  Focused command projections are checked in
  [`human`](../../examples/specification/run/schema-runtime-refinement-validation-human/)
  and
  [`JSON`](../../examples/specification/run/schema-runtime-refinement-validation-json/)
  forms. Imported public-alias resolution and terminal-declaration paths are
  checked by
  [`schema-runtime-refinement-validation-alias`](../../examples/specification/run/schema-runtime-refinement-validation-alias/).
- Format-neutral refinement encode behavior is checked by
  [`adt-variant-refinement-format-neutral-schema-encode`](../../examples/specification/run/adt-variant-refinement-format-neutral-schema-encode/)
  and
  [`adt-variant-refinement-format-neutral-schema-encode-json`](../../examples/specification/check/adt-variant-refinement-format-neutral-schema-encode-json/).
  Source-less refinement rejection is checked by
  [`adt-variant-refinement-format-neutral-schema-encode-source-boundary`](../../examples/specification/check/adt-variant-refinement-format-neutral-schema-encode-source-boundary/).
  Exact-companion access to a private refined schema is checked by
  [`companion-private-schema-access`](../../examples/specification/check/companion-private-schema-access/).
- Format-neutral typed pass-through decode behavior is checked by
  [`adt-variant-refinement-format-neutral-schema-decode`](../../examples/specification/run/adt-variant-refinement-format-neutral-schema-decode/)
  and
  [`adt-variant-refinement-format-neutral-schema-decode-json`](../../examples/specification/check/adt-variant-refinement-format-neutral-schema-decode-json/).
  Its public/private helper boundary is checked together with encode visibility
  by
  [`adt-variant-refinement-format-neutral-schema-encode-visibility`](../../examples/specification/check/adt-variant-refinement-format-neutral-schema-encode-visibility/).
  The positive capability-admission boundary is checked independently by
  [`adt-variant-refinement-decode-capable`](../../examples/specification/check/adt-variant-refinement-decode-capable/).
- Unsupported generated binary decoder capabilities, including singleton,
  union, and innermost nested selection, are checked in human and JSON form by
  [`adt-variant-refinement-decode-capability-human`](../../examples/specification/check/adt-variant-refinement-decode-capability-human/)
  and
  [`adt-variant-refinement-decode-capability-json`](../../examples/specification/check/adt-variant-refinement-decode-capability-json/).
- JVM networking execution evidence compiles checked programs with the
  exported standard modules. The backend cases exercise real socket and task
  cancellation through `net::IO` and `net::system()`. Separate Veln support
  checks nominal handler dispatch, creating-provider retention, host-operation
  order, option and byte forwarding, translated failures, retry state, and
  cleanup-failure unwinding without substituting fake behavior in production
  handlers.
- Normal completion, nested unwinding, registration-time snapshots, and the
  public test boundary are checked by
  [`deferred-cleanup-runtime-boundary`](../../examples/specification/run/deferred-cleanup-runtime-boundary/),
  [`deferred-cleanup-unwind-boundaries`](../../examples/specification/run/deferred-cleanup-unwind-boundaries/),
  and
  [`deferred-cleanup-runtime-boundary-json`](../../examples/specification/test/deferred-cleanup-runtime-boundary-json/).
- Human and JSON execution cases check cleanup-failure aggregation for a
  successful body in
  [`deferred-cleanup-success-cleanup-failure`](../../examples/specification/run/deferred-cleanup-success-cleanup-failure/)
  and
  [`deferred-cleanup-success-cleanup-failure-json`](../../examples/specification/run/deferred-cleanup-success-cleanup-failure-json/),
  and for an already-failing body in
  [`deferred-cleanup-failure-order-human`](../../examples/specification/run/deferred-cleanup-failure-order-human/)
  and
  [`deferred-cleanup-failure-order-json`](../../examples/specification/run/deferred-cleanup-failure-order-json/).
- Cancellation ordering and related cleanup failures are checked by
  [`deferred-cleanup-task-cancellation`](../../examples/specification/run/deferred-cleanup-task-cancellation/),
  [`deferred-cleanup-task-cancellation-failure-human`](../../examples/specification/run/deferred-cleanup-task-cancellation-failure-human/),
  and
  [`deferred-cleanup-task-cancellation-failure-json`](../../examples/specification/run/deferred-cleanup-task-cancellation-failure-json/).
- Signed remainder in runtime contract predicates and its zero-divisor JSON
  failure paths are checked by
  [`contract-remainder`](../../examples/specification/run/contract-remainder/)
  for successful signed, endpoint, and large-divisor results,
  [`contract-remainder-failure`](../../examples/specification/run/contract-remainder-failure/)
  for a violated dynamic predicate,
  [`contract-remainder-mixed-failure`](../../examples/specification/run/contract-remainder-mixed-failure/)
  for integer division after a remainder, and
  [`contract-remainder-zero-json`](../../examples/specification/run/contract-remainder-zero-json/).

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
