# Names And Effects

This is the routing page for implemented name resolution, effect checking, and
compiler-known calls.

## Read First

- Namespaces, shadowing, duplicate checks, module ownership, external package
  imports, and manifest export checks:
  [names-effects-full.md](names-effects-full.md#name-resolution).
- Declaration effect spelling, effect labels, and effect inference:
  [names-effects-full.md](names-effects-full.md#effect-labels) and
  [names-effects-full.md](names-effects-full.md#concurrency-calls).
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
  `net::stream_local_addr` and `net::stream_peer_addr` endpoint text
  inspection for accepted and connected `NetStream` handles,
  fixture-backed stream writes,
  stream close recording, and listener close recording,
  opt-in production loopback socket ownership for listen, sequential accepts,
  client connects, source-visible listener/client connect pairing, reads,
  writes, clean listener end, stream close, and listener close under the same
  public calls,
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
  deadline expiry, or cancellation before the list is fully written. The
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
  under the existing `net` effect boundary. The production multi-chunk routing
  lifecycle accepts one production stream, preserves
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
  that handler boundary effect-free. A
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
- Prelude helper signatures, value semantics, source-backed helper set, and
  descriptor-only helper boundary:
  [names-effects-full.md](names-effects-full.md#prelude-helpers).
- Source-visible `StreamInput`, `DecodeStep<T>`, `DecodeReadiness`,
  `DecodeError`, `EncodeStep<TState>`, and `EncodeError` ADTs plus pure byte
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
  `Int` exact-width fields and `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`,
  `Flag24le`, `Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`,
  `Flag48le`, `Flag56be`, `Flag56le`, `Flag64be`, or `Flag64le` bitset
  fields; compatibility generated decode-step helpers expose the same value
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
  Pure source-backed prelude helpers
  `flag8_is_set`, `flag8_set`,
  `flag8_bits`, `flag8_from_bits`, `flag16be_is_set`, `flag16be_set`,
  `flag16be_bits`, `flag16be_from_bits`, `flag16le_is_set`,
  `flag16le_set`, `flag16le_bits`, `flag16le_from_bits`,
  `flag24be_is_set`, `flag24be_set`, `flag24be_bits`,
  `flag24be_from_bits`, `flag24le_is_set`, `flag24le_set`,
  `flag24le_bits`, `flag24le_from_bits`,
  `flag32be_is_set`, `flag32be_set`, `flag32be_bits`,
  `flag32be_from_bits`, `flag32le_is_set`, `flag32le_set`,
  `flag32le_bits`, `flag32le_from_bits`, `flag40be_is_set`,
  `flag40be_set`, `flag40be_bits`, `flag40be_from_bits`,
  `flag40le_is_set`, `flag40le_set`, `flag40le_bits`,
  `flag40le_from_bits`, `flag48be_is_set`,
  `flag48be_set`, `flag48be_bits`, `flag48be_from_bits`,
  `flag48le_is_set`, `flag48le_set`, `flag48le_bits`,
  `flag48le_from_bits`, `flag56be_is_set`, `flag56be_set`,
  `flag56be_bits`, `flag56be_from_bits`, `flag56le_is_set`,
  `flag56le_set`, `flag56le_bits`, `flag56le_from_bits`,
  `flag64be_is_set`, `flag64be_set`,
  `flag64be_bits`, `flag64be_from_bits`, `flag64le_is_set`,
  `flag64le_set`, `flag64le_bits`, and `flag64le_from_bits` require no
  effects. The checked
  bit-index helpers return `Result` values for invalid indexes, and the
  raw-bit constructors return `Result` values for out-of-range integers.
  Generated binary schema encode
  helpers for the exact-width
  primitive, `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`,
  `Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
  `Flag56be`, `Flag56le`, `Flag64be`, `Flag64le`, supported reserved-bit,
  length-bounded
  `ByteView`, bounded repeated fields written as `Repeat(count, Payload)` or
  `[Payload; count]`, closed dispatch, extension dispatch, and eligible nested
  dispatch payload slices accept schema-local visible fields, using
  `ByteView` fields for length-bounded payloads, `List<T>` fields for repeated
  payloads, and `SchemaDispatchPayload<T>` for extension dispatch payload
  fields. Eligible nested dispatch payloads include public imported binary
  schemas with quotient-sized `ByteView(left_length / right_length)` fields
  whose operands are earlier visible `Int` fields in the nested payload
  schema. Same-module recursive dispatch payload slices with a length-bounded
  recursive field and a primitive base case expose that finite primitive
  payload shape. Lowercase dispatch payload spelling normalizes exact-width
  `uint...` and `flag...` payloads to the same helper behavior as compatible
  upper-case exact-width primitive payloads, and byte-aligned
  `uint... reserves <value>` payloads validate or emit fixed bytes while
  exposing `()` as the payload value. One
  supported reserved-bit slice omits `ReservedBits(2, 0)` or
  `ReservedBits(9, 0)` immediately before `UInt8` from the encode value
  record while exposing the visible byte field. Generated encode helpers
  return `Result<ByteChunk, EncodeError>`.
  `UInt16le`, `UInt24le`, `UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`,
  `UInt56le`, and `UInt64le` fields use little-endian byte order in generated
  decode and encode helpers.
  Source-visible byte helpers also expose checked `u40`, `u48`, and `u64`
  big-endian and little-endian reads and writes through ordinary `Int` values;
  eight-byte values above the source-visible `Int` maximum fail instead of
  wrapping. The `u40` helpers use the same five-byte representations as
  `UInt40be` and `UInt40le`, the `u48` helpers use the same six-byte
  representations as `UInt48be` and `UInt48le`, and the `u64` helpers use the
  same eight-byte representations as `UInt64be` and `UInt64le`.
  HTTP/2 frame-header decoding should be expressed at source level through
  explicit schema operations or ordinary protocol wrapper functions. Remaining
  compiler-known compatibility and protocol diagnostic projection helpers,
  including stream id domain and post-GOAWAY stream failure projection plus
  request and response header-list validation, are listed with those
  signatures:
  [standard byte ADTs](names-effects-full.md#standard-byte-adts) and
  [helper signatures](names-effects-full.md#helper-signatures).
- Descriptor-backed standard symbols, including network stream write-side
  shutdown, source metadata, and the compiler-support source-loading trial:
  [names-effects-full.md](names-effects-full.md#compiler-known-descriptor-table).

## Fast Routes

- Confirming source-backed versus descriptor-only status before proposal work:
  [names-effects-full.md](names-effects-full.md#source-backed-boundary).
- Checking self-hosting migration completion before new proposal work:
  [names-effects-full.md](names-effects-full.md#source-backed-boundary).
  The migration is complete when the descriptor-only pure-helper list is empty
  and all compiler-known pure helpers in that split are source-backed.
  Completion history:
  [../reference/implemented-proposals/self-hosting-standard-library.md](../reference/implemented-proposals/self-hosting-standard-library.md).
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
