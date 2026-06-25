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
  values, fixture-backed stream writes, stream close recording, and listener
  close recording,
  opt-in production loopback socket ownership for listen, sequential accepts,
  reads, writes, clean listener end, stream close, and listener close under
  the same public calls,
  relative deadline calls, and cancellable deadline waits through
  source-visible `CancelToken` handles. `time::is_cancelled` observes a token
  as `Bool` under the same `time` effect without waiting or requesting
  cancellation. The value-returning cancellable wait returns
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
  `net` effect. `net::write_chunks_until_cancellable` combines those
  boundaries under `net` and `time`, writing the source list in order and
  returning ordinary write outcomes for full completion, deadline expiry, or
  cancellation before the list is fully written. The adapter-owned outbound
  ordering example accepts
  deterministic loopback streams, routes ordinary `StreamInput` values
  through a channel, calls multiple pure handler functions, combines their
  ordinary `ResponseAction` values into one adapter-owned order, and projects
  only `SendBytes` actions to ordered `net::write_chunks` calls while
  declaring `net` and `concurrency`; the handlers stay free of transport
  effects. The cancellable channel-first routing case uses
  receiver-list selection before the wait outcome and keeps the same adapter
  effect boundary.
  Malformed receive fixtures, failed send, write, stream close, or listener
  close recording, forced accept, read, write, or close failures, forced
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
  transport effects.
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
  fixture events to owned host streams. The two-stream adapter
  lifecycle accepts two independent production streams from one listener,
  routes each stream through the same ordinary handler/action boundary, writes
  only ordered `SendBytes` actions, closes each stream, and observes clean
  listener end without adding public calls or effect labels. The
  listener-drain adapter uses the same public calls and effect declarations
  while recursively accepting configured production streams until
  `net::accept_or_end` reports clean listener end; forced production read
  failure on that path remains a runtime transport failure. Forced production
  accept or read failures through the deadline-aware calls remain runtime
  transport failures under the same coarse effect labels.
  The channel-first stream routing examples use two, three, and four typed
  `StreamInput` channels plus existing channel selection. Receiver-list
  five-route through thirty-route examples use
  `channel::select_many_priority` on a non-empty
  `List<Receiver<StreamInput>>`, and the timeout selection example uses
  `channel::select_many_timeout` and
  `channel::select_many_timeout_result` to preserve receiver-list priority
  while returning `None` or `Ok(None)` when no route is ready before the
  timeout, before invoking a plain handler. Cancellable receiver-list timeout
  selection uses `channel::select_many_timeout_cancellable` with the same
  selected value shape and returns `Err(SelectError)` when its
  source-visible `CancelToken` wins before a ready receiver. The two-receiver
  cancellable timeout helper `channel::select_timeout_cancellable` uses the
  same selected value shape, left/right indexes, timeout behavior, and token
  cancellation boundary. A cancellable channel-first adapter composes
  receiver-list routes with
  `time::wait_until_cancellable_outcome`; these cancellable adapter paths
  declare `time` and `concurrency` while the handler boundary remains free of
  transport effects. Other routing adapters require `concurrency`, and socket
  wrappers around them require both `net` and `concurrency`.
- Receiver-list channel-first routing effect coverage includes the general
  helper shape over `List<Receiver<StreamInput>>`: channel selection carries
  `concurrency`, while the selected stream handler remains effect-free.
  Earlier route-count examples remain bounded evidence, not a template for
  adding further same-shaped fixtures.
- Prelude helper signatures, value semantics, source-backed helper set, and
  descriptor-only helper boundary:
  [names-effects-full.md](names-effects-full.md#prelude-helpers).
- Source-visible `StreamInput`, `DecodeStep<T>`, `DecodeReadiness`,
  `DecodeError`, `EncodeStep<TState>`, and `EncodeError` ADTs plus pure byte
  vocabulary helpers for `Byte`, `ByteChunk`, `ByteView`, `ByteCount`,
  `ByteOffset`, compact hex fixture decoding, visible ASCII conversion between
  strings and immutable chunks, bounded `ByteView` slicing, outgoing
  `List<ByteChunk>` construction, fixed-width unsigned big-endian and
  little-endian reads and writes, fixed U8 field checks, and the narrow schema
  width-sample decode helper. The implemented pending-input slice
  appends incoming `StreamInput.Chunk` bytes into immutable retained
  `ByteChunk` values, bounds retention with `ByteCount`, uses `ByteView`
  prefixes for parsing, tracks absolute `ByteOffset` separately, and collects
  outgoing immutable chunks in `List<ByteChunk>` action values. Generated
  binary schema decode helpers return schema-local value fields, including
  `Int` exact-width fields and `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`,
  `Flag24le`, `Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`,
  `Flag48le`, `Flag56be`, `Flag56le`, `Flag64be`, or `Flag64le` bitset
  fields, unless the eligible
  structural `map to Target` slice, including decoded-field selected mappings
  that resolve to one record shape, resolves a mapped record shape; generated
  decode-step helpers expose the same value shape through `DecodeStep<T>` for
  open input. Pure source-backed prelude helpers `flag8_is_set`, `flag8_set`,
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
  `ByteView`, closed dispatch, extension dispatch, and eligible nested
  dispatch payload slices accept schema-local visible
  fields, using `ByteView` fields for length-bounded payloads and
  `SchemaDispatchPayload<T>` for extension dispatch payload fields. One
  supported reserved-bit slice omits `ReservedBits(2, 0)` or
  `ReservedBits(9, 0)` immediately before `UInt8` from the encode value
  record while exposing the visible byte field. One
  unselected structural mapping can instead accept a mapped record shape for
  direct field projections or a direct ADT constructor whose payloads are
  schema-local visible fields already supported by the generated encode
  helper, or whose single payload is a record expression over those fields.
  Generated encode helpers return `Result<ByteChunk, EncodeError>`.
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
  HTTP/2 frame-header decoding is provided by generated schema helpers such
  as `byte_decode_http2_frame_header_wire`; bounded payload frame decoding and
  protocol diagnostic projection helpers including stream id domain and
  post-GOAWAY stream failure projection, plus request and response header-list
  validation, are
  listed with those signatures:
  [standard byte ADTs](names-effects-full.md#standard-byte-adts) and
  [helper signatures](names-effects-full.md#helper-signatures).
- Descriptor-backed standard symbols, source metadata, and the
  compiler-support source-loading trial:
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
