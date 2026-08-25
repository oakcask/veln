---
role: specification
authority: normative
update-when: The documented name resolution, effect behavior, or executable names/effects evidence changes.
---

# Names And Effects

This is the routing page for implemented name resolution, effect checking, and
compiler-known calls.

## Read First

- Source ADT type and constructor names start with an ASCII uppercase letter.
  Function, test, public function-alias, parameter, result, local `let`, pattern,
  handler parameter, operation-clause parameter, and hole `satisfy` binding
  names start with an ASCII lowercase letter. Public type-alias names follow the
  type rule. A violation reports `name.invalid_case` at the exact written token
  and prevents checked-core and typed-IR output. An underscore-led token is
  retained for recovery in these positions; standalone `_` remains a wildcard
  or discard. The checked `identifier-casing-source-recovery-json` and
  `identifier-casing-source-recovery-human` cases define exact JSON and human
  diagnostics for the declaration rows and the function parameter, return
  binding, and local `let` binding rows. The checked
  `identifier-casing-binding-positions-json` and
  `identifier-casing-binding-positions-human` cases define exact diagnostics
  for handler parameters, operation-clause parameters, pattern-head bindings,
  and hole `satisfy` bindings. The checked
  `identifier-casing-underscore-recovery-human` and
  `identifier-casing-underscore-recovery-json` cases define underscore-led
  parser recovery without missing-name cascades. Invalid value bindings are
  kept out of normal local lookup and repair candidates; a unique same-function
  recovery record suppresses only derivative unresolved-name diagnostics. The
  run cases
  `identifier-casing-reachable-recovery`,
  `identifier-casing-reachable-invalid-alias`,
  `identifier-casing-reachable-expression-type`, and
  `identifier-casing-unreachable-peer` define selected-entry reachability,
  including reachable invalid public function aliases and expression-only
  constructor/type references, plus the rule that reachable local value
  spellings and type references do not make unrelated invalid declarations
  reachable.
  The checked `identifier-casing-import-recovery-isolation-json` and
  `identifier-casing-public-alias-recovery-isolation-json` cases fix that
  quarantined recovery records do not cross import boundaries or satisfy public
  alias targets. The LSP single-file diagnostics helper reports the same
  parse-clean source casing diagnostics for that source. Workspace snapshot and
  open-document overlay selection evidence remains outside this source
  foundation.

- Namespaces, shadowing, duplicate checks, module ownership, external package
  imports, and manifest export checks:
  [names-effects-full.md](names-effects-full.md#name-resolution).
- Declaration effect spelling, effect-row substitution, host effect labels,
  nominal operation effects, lexical handlers, and effect inference:
  [names-effects-full.md](names-effects-full.md#effect-labels) and
  [names-effects-full.md](names-effects-full.md#concurrency-calls).
  Task creation preserves the concrete effect set of its job callable; the
  checked `http2-service-task-effect-row` case fixes the current task
  effect-row boundary.
- Companion private-function calls observe the established signature and
  effects of the exact target function without changing the target's inferred
  production effects. The checked
  `companion-private-function-established-effects` and
  `companion-private-function-established-effects-missing` cases fix the
  companion declaration boundary.
- A `.test.veln` companion with an explicit target import may name private
  target nominal effects through qualified target paths in `perform`,
  declaration effect lists, function type annotation effect lists, and
  companion-local handler `handles` clauses and declared handler effect
  lists. The checked
  `companion-private-effect-*` cases fix the exact-target and isolation
  boundaries.
- A `.test.veln` companion with an explicit target import may use a private
  target handler through a qualified target path in `handle ... with ...`.
  The handler's handled effect and retained effects are established from the
  production target before companion checking. The checked
  `companion-private-handler-*` cases fix the exact-target, isolation, and
  declaration boundaries.
- Private `std::diagnostic` ownership, public prelude aliases, and the nested
  HTTP/2 and HPACK diagnostic detail types:
  [Prelude Helpers](names-effects-full.md#prelude-helpers).
- Compiler-known calls:
  [stdio](names-effects-full.md#stdio-calls),
  [file-system](names-effects-full.md#file-system-calls),
  [network and time](names-effects-full.md#network-and-time-boundary-calls),
  [process](names-effects-full.md#process-calls), and
  [concurrency](names-effects-full.md#concurrency-calls).
  Prelude byte and protocol helper signatures, including HTTP/2 runtime
  diagnostic payload helpers, are listed in
  [Helper Signatures](names-effects-full.md#helper-signatures).
  The network and time boundary keeps the coarse `net` and `time` effect
  labels and includes descriptor-backed chunk calls, fixture-backed listener
  and stream calls, optional clean-end listener accepts and stream reads,
  deadline-aware listener accepts that return `None` on accept deadline
  expiry, cancellable deadline-aware listener accepts that return
  `AcceptStream`, `AcceptEnd`, `AcceptDeadlineExpired`, or `AcceptCancelled`
  values, deadline-aware stream reads that return `None` on read deadline
  expiry or clean stream end, cancellable deadline-aware stream reads that
  return `ReadChunk`, `ReadEnd`, `ReadDeadlineExpired`, or `ReadCancelled`
  values, deadline-aware stream writes that return `WriteCompleted` or
  `WriteDeadlineExpired`, cancellable deadline-aware stream writes that
  return `WriteCompleted`, `WriteDeadlineExpired`, or `WriteCancelled`
  values, source-visible `net::connect` client streams,
  `net::listener_local_addr` endpoint text inspection for `NetListener`
  handles,
  `net::stream_local_addr` and `net::stream_peer_addr` endpoint text
  inspection for accepted and connected `NetStream` handles,
  `net::stream_can_read`, `net::stream_can_write`, and
  `net::stream_is_closed` stream state inspection for owned `NetStream`
  handles,
  fixture-backed stream writes,
  stream read-side shutdown, stream close recording, and listener close
  recording,
  structured runtime failures for listen, connect, accept, read, write,
  shutdown, stream close, and listener close with no finer-grained effect
  labels,
  adapter code that may inspect known stream endpoints and then return a
  pure protocol diagnostic without that result being reclassified as a
  transport failure; the checked
  `socket-stream-adapter-protocol-precedence-json` case fixes this precedence,
  opt-in production loopback socket ownership for listen, sequential accepts,
  client connects, source-visible listener/client connect pairing, reads,
  writes, clean listener end, stream read-side shutdown, stream close, and
  listener close under the same public calls,
  opt-in external host socket ownership for listeners and connections that
  are not paired or synthesized by the current runtime, with the same public
  handles, lifecycle calls, structured failures, and coarse effects,
  relative and absolute monotonic deadline calls, and cancellable deadline
  waits through
  source-visible `CancelToken` handles. `time::cancel_owner` creates a
  source-visible cancellation owner, `time::cancel_token_from` exposes an
  observer token for existing cancellable operations, and
  `time::cancel_owned` requests cancellation through the owner under the
  same `time` effect. `time::is_cancelled` observes a token
  as `Bool` under the same `time` effect without waiting or requesting
  cancellation, and `time::is_cancelled_owner` observes an owner directly
  under that same boundary. `time::monotonic_ms` returns a host-owned monotonic
  millisecond counter under the same `time` effect for elapsed-time
  measurement without exposing wall-clock dates. `time::deadline_at_ms`
  constructs a `Deadline` from an absolute monotonic millisecond value in the
  same clock domain, so existing deadline-aware waits, accepts, reads, and
  writes consume relative and absolute deadlines through the same path.
  Direct `time::cancel`
  remains available for direct tokens created by `time::cancel_token`, while
  owner-derived observer tokens reject direct cancellation at runtime. The
  value-returning cancellable wait returns
  `CancellableWaitOutcome` under the same `time` effect so adapter code can
  treat completion, deadline expiry, and cancellation as ordinary values.
  `StreamAdapterAction` exposes source-visible adapter response actions:
  `SendBytes(ByteChunk)`, `EndStream`, and `Ignore`.
  `stream_adapter_drain_actions(stream, handler)` accepts an owned
  `NetStream` and a pure `fn(StreamInput) -> List<StreamAdapterAction>`
  handler, drains optional stream reads through a channel-routed
  `StreamInput` boundary, and writes only ordered `SendBytes` chunks through
  `net::write_chunks`; it requires `net` and `concurrency`.
  The exported `transport` module declares `DuplexStream` with
  `read_chunk() -> Option<ByteChunk>` and
  `write_chunks(chunks: List<ByteChunk>) -> ()`. The exported
  `transport::net::net_stream(stream)` lexical handler handles that effect
  through one caller-owned `NetStream`, removes the duplex-stream effect from
  the handled expression, and adds only the existing `net` effect. The checked
  `http2-connection-transport-handler-effects` case fixes this static
  replacement boundary.
  `http2::connection::drive_server` and
  `http2::connection::drive_client` expose only `transport::DuplexStream`.
  `http2::connection::drive_server_application<effect E>` exposes the same
  single duplex-stream effect and preserves the application callback row:
  `handler` has type
  `fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>
  effects [...E]`, and the driver requires
  `[std::transport::DuplexStream, ...E]`. Handling the driver with
  `transport::net::net_stream` replaces only the duplex-stream effect with
  `net`; callback effects such as `db` remain required by the handled
  expression. The checked `http2-service-transport-effect-replacement` case
  fixes the real application-driver and `serve_connection` effect boundary.
  `http2::connection::serve_connection<effect E>` has the same callback shape
  and requires `[std::transport::DuplexStream, ...E]`.
  `http2::connection::serve_tcp<effect E>` accepts an owned `NetListener`,
  installs `transport::net::net_stream` inside each spawned connection task,
  and requires `[net, concurrency, ...E]`. A pure TCP service callback
  therefore requires only `net` and `concurrency`; callback effects such as
  `db` remain required. The checked `http2-service-task-effect-row` and
  `http2-service-task-handler-boundary` cases fix the TCP service and task
  handler-inheritance boundaries. Handling any connection driver with
  `net_stream` preserves caller ownership of listen, accept, close, deadline,
  cancellation, and task behavior outside the HTTP/2 driver.
  `http2::connection::request_endpoint_sequence<effect E>` accepts a finite
  request list and a response callback with type
  `fn(Http2ClientResponse) -> Result<(), String> effects [...E]`. Its public
  effect boundary is `[net, concurrency, ...E]`. A pure client response
  callback therefore requires only `net` and `concurrency`; callback effects
  such as `db` remain required. The checked
  `http2-client-service-effect-row` case fixes the pure and effectful client
  callback boundaries.
  `stream_adapter_accept_loop(listener, handler)` accepts an owned
  `NetListener` and the same pure handler shape, repeatedly accepts streams
  until clean listener end, delegates each accepted stream to
  `stream_adapter_drain_actions`, closes each accepted stream, closes the
  listener after clean end, and requires `net` and `concurrency`.
  `stream_adapter_drain_actions_until_cancellable(stream, handler, deadline,
  token)` uses the same adapter-owned drain, route, and pure handler boundary,
  then writes only ordered `SendBytes` chunks through
  `net::write_chunks_until_cancellable`; it requires `net`, `time`, and
  `concurrency`, and returns `StreamWriteOutcome` for full completion,
  deadline expiry, or cancellation before all projected chunks are written.
  Stream adapter routing that combines those outcomes with channel-routed
  `StreamInput` values declares both `time` and `concurrency`; the handler it
  calls stays free of transport effects. Socket lifecycle routing can combine
  an accepted `NetStream`, ordinary `net::read_chunk` input, channel routing,
  and a value-returning cancellable wait under `net`, `time`, and
  `concurrency` while keeping the handler pure. A cancellable deadline-aware
  lifecycle adapter composes `net::accept_until_cancellable`,
  `net::read_chunk_until_cancellable`, ordinary channel routing, and ordered
  `net::write_chunk` projection under the same `net`, `time`, and
  `concurrency` boundary. `net::write_chunk_until` requires `net` and
  `time`, writes one immutable `ByteChunk` before the deadline, and
  preserves host write failures as runtime failures while exposing deadline
  expiry as an ordinary write outcome value. `net::write_chunk_until_cancellable`
  requires `net` and `time` and preserves host write failures as runtime
  failures while exposing deadline expiry and cancellation as ordinary write
  outcome values. `net::write_chunks` writes a source-owned
  `List<ByteChunk>` to a `NetStream` in list order under the same coarse
  `net` effect. `net::write_chunks_until` combines the ordered chunk-list
  write path with deadline observation under `net` and `time`, writing the
  source list in order and returning ordinary write outcomes for full
  completion or deadline expiry before the list is fully written.
  `net::write_chunks_until_cancellable` extends that boundary with
  cancellation, returning ordinary write outcomes for full completion,
  deadline expiry, or cancellation before the list is fully written.
  `net::shutdown_write` shuts down the stream write side under the same
  coarse `net` effect, makes later writes fail as runtime transport failures,
  and leaves the clean read-end path available on the same `NetStream`.
  `net::shutdown_read` shuts down the stream read side under the same coarse
  `net` effect, makes later optional reads observe clean end, and leaves the
  write side owned by the same `NetStream`. `net::stream_can_read`,
  `net::stream_can_write`, and `net::stream_is_closed` observe read-side,
  write-side, and full-close state as `Bool` under that same `net` effect
  without consuming stream ownership. After full close, state inspection can
  still observe the closed handle while later read, write, or shutdown
  transport operations on that stale handle fail as runtime transport
  failures. The
  adapter-owned outbound
  ordering example accepts
  deterministic loopback streams, routes ordinary `StreamInput` values
  through a channel, calls multiple pure handler functions, combines their
  ordinary `ResponseAction` values into one adapter-owned order, and projects
  only `SendBytes` actions to ordered `net::write_chunks` calls while
  declaring `net` and `concurrency`; the handlers stay free of transport
  effects. The cancellable channel-first routing case uses receiver-list
  cancellable timeout selection to map routed, timed-out, and cancelled
  outcomes into ordinary adapter completion values before producing adapter
  action values while keeping the same adapter effect boundary.
  Malformed receive fixtures, failed send, write, stream close, or listener
  close recording, forced connect, accept, read, write, or close failures, forced
  timeout or deadline expiry through runtime-failure waits, and forced
  cancellable-wait cancellation through the runtime-failure wait are runtime
  failures. Explicit listener close keeps already accepted streams owned by
  their `NetStream` handles, while any later `net::accept`,
  `net::accept_or_end`, `net::accept_until`, or
  `net::accept_until_cancellable` call on that listener fails as a runtime
  transport failure. Forced accept failure
  through the deadline-aware optional and cancellable accept paths and forced
  read or write failure through the deadline-aware optional,
  deadline-aware write, and cancellable stream paths also stay runtime
  failures.
  The socket stream adapter routing context example composes an ordinary
  event value, explicit state, route metadata, and trace metadata into one
  anonymous record passed through `task::spawn_with<Result, Context>`. The
  adapter side declares `concurrency`; the handler receives one context
  parameter, returns ordinary action intent values, and stays free of
  transport effects. Dedicated socket lifecycle examples additionally call
  `net::close_stream` after ordered writes or cancellation cleanup. The clean
  shutdown adapter also closes the owned `NetListener` after projecting only
  `SendBytes` actions, so cancellation and deadline-expiry decisions do not
  emit extra bytes. These examples add no new effect label or compiler-known
  routing call. The owned-lifecycle and close-lifecycle adapters declare
  `net` and `concurrency`; the deadline-aware, cancellable, cancellable
  deadline-aware, cancel-close, and clean-shutdown lifecycle adapters declare
  `net`, `time`, and `concurrency`; the pure handler boundary remains free of
  transport effects. The cancellation-owner lifecycle adapter keeps the
  `CancelOwner` in adapter cleanup, passes only the observer `CancelToken` to
  routing, wait, and read code, and observes both `WaitCancelled` and
  `ReadCancelled` as ordinary outcome values after owner-requested
  cancellation.
  The production loopback lifecycle cases use the same `net` and
  `concurrency` declarations as the close-lifecycle adapter, and the
  production deadline-aware lifecycle adds the existing coarse `time` label
  because it composes `net::accept_until`, `net::read_chunk_until`, ordinary
  channel routing, ordered writes, and explicit close. The production
  cancellable deadline-aware lifecycle uses the same `net`, `time`, and
  `concurrency` boundary with `net::accept_until_cancellable`,
  `net::read_chunk_until_cancellable`, ordinary channel-routed `StreamInput`
  values, ordered `SendBytes` projection, explicit stream close, clean
  listener end, and explicit listener close. The runtime path changes from
  fixture events to owned host streams. The production owner-drain adapter
  keeps cancellation authority in adapter cleanup through `CancelOwner`,
  passes only observer `CancelToken` values to cancellable deadline-aware
  accept/read and channel-routing code, drains production streams until clean
  listener end or accept cancellation, and checks that owner cancellation
  after one accepted stream event makes a later stream read return
  `ReadCancelled` before another handler route continues. It projects ordered
  `SendBytes` actions through `net::write_chunks` while requiring the same
  `net`, `time`, and `concurrency` adapter boundary. The two-stream adapter
  lifecycle accepts two independent production streams from one listener,
  routes each stream through the same ordinary handler/action boundary, writes
  only ordered `SendBytes` actions, closes each stream, and observes clean
  listener end without adding public calls or effect labels. A source-visible
  production listen/connect lifecycle opens one production-owned listener from
  an address value, connects a source-owned client stream through the same
  value, accepts the paired server stream, exchanges one byte chunk, closes
  both stream handles, observes clean listener end, and closes the listener
  under the existing `net` effect boundary. The production read-side shutdown
  lifecycle shuts down accepted-stream input, observes clean optional read
  end, writes response bytes, and then explicitly shuts down write ownership
  and closes the stream under the same `net` effect. The production
  write-side shutdown lifecycle writes response bytes, shuts down output,
  observes clean read end, and then closes the stream under the same `net`
  effect. The production stream state inspection case observes `NetStream`
  read, write, and closed status before shutdown, after read-side shutdown,
  after write-side shutdown, and after close, while preserving stream
  ownership for the intervening write and close operations. A companion
  production stale-handle case confirms that a write after observed close
  fails as a runtime transport failure. The production
  multi-chunk routing lifecycle accepts one production stream, preserves
  configured read chunk boundaries as repeated `net::read_chunk_or_end`
  results, routes each chunk as an ordinary `StreamInput.Chunk` through an
  existing channel to a pure handler, and projects ordered `SendBytes` actions
  through `net::write_chunks` while requiring the same `net` and
  `concurrency` adapter boundary. The multi-event adapter task-helper variant
  routes each accepted stream event through the same channel boundary and then
  through `task::spawn_with<Result, Context>` via an adapter-owned task helper.
  The helper carries adapter-owned route and trace metadata, preserves event
  sequence before projecting ordered `SendBytes` actions through
  `net::write_chunks`, and calls a pure event/action handler that receives no
  `NetStream`. A per-stream handler failure returned from that task boundary
  can be represented as an ordinary adapter-owned action value; the adapter
  closes the accepted stream, observes deterministic listener end, and does
  not project later response bytes for the failed stream. The matching effect
  fixture rejects adapter entry points that omit either label while keeping
  that handler boundary effect-free. The concurrent stream task-drain adapter
  retains accepted streams and `Task<Result<HandlerOutput, String>>` handles
  in one recursive pending-work value until clean listener end, then joins,
  writes, and closes in acceptance order. The adapter requires `net` and
  `concurrency`; the ordinary-context application handler remains effect-free,
  and a handler `Err` suppresses writes only for its stream. A separate
  fail-fast adapter case uses the same recursive pending-work shape and the
  same `net` and `concurrency` effects. After the first handler `Err` or task
  join failure, it calls `task::cancel` and then `task::join` for every later
  task, closes every retained stream once, and suppresses all later response
  writes while the ordinary handler remains effect-free. A
  standard stream routing helper case uses `stream_adapter_drain_actions` to
  drain one accepted production stream into ordered `StreamAdapterAction`
  values, filters response projection to `SendBytes` chunks through
  `net::write_chunks`, closes the stream, and then observes clean listener end
  under the existing `net` and `concurrency` effects. The accept-loop helper
  case uses `stream_adapter_accept_loop` to own the
  listener lifecycle, route at least two accepted production streams through
  the same pure handler boundary, project only `SendBytes` chunks, close each
  stream, close the listener after clean listener end, and reject callers that
  omit either `net` or `concurrency`. The cancellable
  adapter write-drain helper case uses
  `stream_adapter_drain_actions_until_cancellable` to keep the same pure
  handler and channel-routed `StreamInput` boundary while projecting ordered
  `SendBytes` chunks through `net::write_chunks_until_cancellable`; checked
  cases cover completion, deadline expiry, cancellation, and the required
  `net`, `time`, and `concurrency` effects. A
  forced production read failure on the same multi-chunk
  routing path remains a runtime transport failure after the stream is
  accepted and before any chunk routing, response writes, stream close, or
  clean listener end is recorded. The two-stream multi-cycle routing case
  combines the same
  adapter-owned socket and channel boundary with more than one accepted
  production stream and repeated per-stream read/route/write cycles; handlers
  still receive only ordinary `StreamInput` values and no `NetStream`. The
  listener-drain adapter uses the same public calls and effect declarations
  while recursively accepting configured production streams until
  `net::accept_or_end` reports clean listener end; forced production read
  failure on that path remains a runtime transport failure. Forced production
  adapter-owned outbound write failure on the ordered `net::write_chunks`
  projection path remains a runtime transport failure after ordinary handler
  routing and before response writes or stream close. Forced production accept
  or read failures through the deadline-aware calls remain runtime transport
  failures under the same coarse effect labels.
  The HTTP/2 adapter/core write boundary keeps the handler and pure core
  send-intent path free of transport effects, while the adapter that accepts a
  `NetStream` and projects accepted HEADERS and DATA chunks through
  `net::write_chunks` requires the existing coarse `net` effect. The matching
  effect fixture rejects an adapter entry point that omits `net` and adds no
  new effect label.
  The channel-first stream routing examples use two, three, and four typed
  `StreamInput` channels plus existing channel selection. The general
  receiver-list example uses `channel::select_many_priority` on a non-empty
  `List<Receiver<StreamInput>>` with more than four routes, and the timeout
  selection example uses
  `channel::select_many_timeout` and
  `channel::select_many_timeout_result` to preserve receiver-list priority
  while returning `None` or `Ok(None)` when no route is ready before the
  timeout, before invoking a plain handler. Cancellable receiver-list timeout
  selection uses `channel::select_many_timeout_cancellable` with the same
  selected value shape and returns `Err(SelectError)` when its
  source-visible `CancelToken` wins before a ready receiver. The two-receiver
  timeout-result helper `channel::select_timeout_result` uses the same
  selected value shape, left/right indexes, rotating tie behavior, timeout
  behavior, fallible selection boundary, and `Int` timeout argument while
  requiring only `concurrency`. The two-receiver
  cancellable timeout helper `channel::select_timeout_cancellable` uses the
  same selected value shape, left/right indexes, timeout behavior, and token
  cancellation boundary. A cancellable channel-first adapter composes
  receiver-list routes with
  `channel::select_many_timeout_cancellable`, translates its routed,
  timed-out, and cancelled results into ordinary source outcome values, and
  then calls the handler only for ordinary stream events; these cancellable
  adapter paths declare `time` and `concurrency` while the handler boundary
  remains free of transport effects. Other routing adapters require
  `concurrency`, socket wrappers around them require both `net` and
  `concurrency`, and cancellable socket wrappers require `net`, `time`, and
  `concurrency`.
- Receiver-list channel-first routing effect coverage includes the general
  helper shape over `List<Receiver<StreamInput>>`: channel selection carries
  `concurrency`, while the selected stream handler remains effect-free.
- Toolchain `std` package loading, implicit prelude imports, helper signatures,
  and compiler adapter boundary:
  [names-effects-full.md](names-effects-full.md#prelude-helpers).
- Source-visible `StreamInput`, `StreamAdapterAction`, `DecodeStep<T>`,
  `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`, and `EncodeError`
  ADTs plus pure byte
  vocabulary helpers for `Byte`, `ByteChunk`, `ByteView`, `ByteCount`,
  `ByteOffset`, compact hex fixture decoding, visible ASCII conversion between
  strings and immutable chunks, bounded `ByteView` slicing, outgoing
  `List<ByteChunk>` construction and budgeted chunk production, fixed-width
  unsigned big-endian and little-endian reads and writes, fixed U8 field
  checks, and the narrow schema width-sample decode helper. The implemented
  pending-input slice
  appends incoming `StreamInput.Chunk` bytes into immutable retained
  `ByteChunk` values, bounds retention with `ByteCount`, uses `ByteView`
  prefixes for parsing, tracks absolute `ByteOffset` separately, and collects
  outgoing immutable chunks in `List<ByteChunk>` action values. Generated
  binary schema decode helpers return schema-local value fields, including
  `Int` exact-width unsigned fields; compatibility generated decode-step
  helpers expose the same value
  shape through `DecodeStep<T>` for open input. Format-neutral schema decode
  helpers accept and return schema-local visible records through
  `Result<T, String>` when all fields are recursive visible shapes made from
  scalar leaves, anonymous record fields, `Option<T>`, `List<T>`, `Vec<T>`,
  and `Dict<String, T>`. `Result<Ok, Err>` is supported when both payloads are
  recursive format-neutral visible shapes. Same-module source ADTs and public
  imported source ADTs referenced through written `use` paths are supported in
  those positions when every constructor payload is a recursive
  format-neutral visible shape.
  Explicit
  `decode Schema from view at base` expressions are the public source surface
  for applying schemas and expose that decode-step shape without naming the
  generated helper in source; imported public schemas may be cited through
  qualified paths, including public schema aliases. Explicit
  `encode Schema from value` expressions are the matching public encode
  surface for schema-local values and accept the same schema-reference paths.
  Format-neutral schema encode helpers accept and return schema-local visible
  records through `Result<T, String>` when every field recursively contains
  only supported scalar leaves, anonymous records, `Option<T>`, `List<T>`,
  `Vec<T>`, `Dict<String, T>`, `Result<Ok, Err>`, and eligible source ADTs.
  Every child and ADT constructor payload must satisfy the same boundary; the
  helper does not produce binary bytes.
  Generated binary schema encode
  helpers for the exact-width
  unsigned primitive, supported reserved-bit,
  length-bounded
  `ByteView`, bounded repeated fields written as `Repeat(count, Payload)` or
  `[Payload; count]`, closed dispatch, extension dispatch, and eligible nested
  dispatch payload slices accept schema-local visible fields, using
  `ByteView` fields for length-bounded payloads, `List<T>` fields for repeated
  visible payloads, no visible field for representation-only repeated reserved
  payloads, and `SchemaDispatchPayload<T>` for extension dispatch payload
  fields. Schema-facing byte conversions are explicit source-visible helper
  calls: `byte_view` supplies bounded `ByteView` payloads over owned bytes, and
  `byte_view_to_chunk` materializes schema-decoded bounded bytes back to owned
  `ByteChunk` data. The executable coverage is
  `../../examples/specification/run/binary-schema-byte-conversion-boundary/`
  and
  `../../examples/specification/run/binary-schema-byte-conversion-range-json/`.
  Eligible nested dispatch payloads include public imported binary
  schemas with quotient-sized `ByteView(left_length / right_length)` fields
  whose operands are earlier visible `Int` fields in the nested payload
  schema. Same-module recursive dispatch payload slices with a length-bounded
  recursive field and a primitive base case expose that finite primitive
  payload shape. Lowercase dispatch payload spelling normalizes exact-width
  `uint...` payloads to the same helper behavior as compatible upper-case
  exact-width primitive payloads, and byte-aligned
  `uint... reserves <value>` payloads validate or emit fixed bytes while
  exposing `()` as the payload value. A direct binary schema
  `ReservedBits(width, value)` immediately before `UInt8` omits the reserved
  field from the encode value record when the width is positive and not byte
  aligned, the value fits that width, and the group with trailing zero padding
  fits in at most eight big-endian bytes. The visible byte remains an ordinary
  `Int` field. Generated encode helpers
  return `Result<ByteChunk, EncodeError>`.
  `UInt16le`, `UInt24le`, `UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`,
  `UInt56le`, and `UInt64le` fields use little-endian byte order in generated
  decode and encode helpers.
  Source-visible byte helpers also expose checked `u40`, `u48`, `u56`, and
  `u64` big-endian and little-endian reads and writes through ordinary `Int`
  values; eight-byte values above the source-visible `Int` maximum fail
  instead of wrapping. The `u40` helpers use the same five-byte
  representations as `UInt40be` and `UInt40le`, the `u48` helpers use the
  same six-byte representations as `UInt48be` and `UInt48le`, the `u56`
  helpers use the same seven-byte representations as `UInt56be` and
  `UInt56le`, and the `u64` helpers use the same eight-byte representations as
  `UInt64be` and `UInt64le`.
  HTTP/2 frame-header decoding should be expressed at source level through
  explicit schema operations or ordinary protocol wrapper functions. Remaining
  compiler-known compatibility and protocol diagnostic projection helpers,
  including stream id domain and post-GOAWAY stream failure projection plus
  request and response header-list validation, are listed with those
  signatures:
  [standard byte ADTs](names-effects-full.md#standard-byte-adts) and
  [helper signatures](names-effects-full.md#helper-signatures).
- Descriptor-backed runtime symbols, including network stream write-side
  shutdown, and the compiler adapter boundary for `std` declarations:
  [names-effects-full.md](names-effects-full.md#compiler-known-descriptor-table).

## Fast Routes

- Checking the toolchain package, bootstrap exception, and compiler adapter
  boundary:
  [names-effects-full.md](names-effects-full.md#standard-package-boundary).
- Package migration completion history:
  [../reference/implemented-proposals/standard-library-package.md](../reference/implemented-proposals/standard-library-package.md).
- Checking helper signatures before changing the prelude adapter:
  [names-effects-full.md](names-effects-full.md#helper-signatures).
- Checking standard symbol descriptor metadata:
  [names-effects-full.md](names-effects-full.md#compiler-known-descriptor-table).

## Read When

- Updating `name.*`, `module.*`, or `effect.*` diagnostics.
- Changing compiler-known calls, reachability, prelude helpers, or effect
  inference.
- Deciding whether a behavior belongs in the implemented reference or remains
  proposal rationale.

## Skip Unless Needed

- Do not open source-decision history before the implemented behavior in
  [names-effects-full.md](names-effects-full.md) answers the question.
- Use [diagnostics-json.md](diagnostics-json.md) only when the machine-readable
  shape of a diagnostic also changes.
