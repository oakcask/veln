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
  expiry, deadline-aware stream reads that return `None` on read deadline
  expiry or clean stream end, fixture-backed stream writes and stream close
  recording, relative deadline calls, and cancellable deadline waits through
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
  `concurrency` while keeping the handler pure. The cancellable channel-first
  routing case uses receiver-list selection before the wait outcome and keeps
  the same adapter effect boundary.
  Malformed receive fixtures, failed send, write, or close recording, forced
  accept, read, write, or close failures, forced timeout or deadline expiry
  through runtime-failure waits, and forced cancellable-wait cancellation
  through the runtime-failure wait are runtime failures. Forced accept failure
  through the deadline-aware optional accept path and forced read failure
  through the deadline-aware optional read path also stay runtime failures.
  The socket stream adapter routing examples compose existing `net` stream
  calls with existing channel and task calls under `concurrency`, including
  optional listener accept, multiple optional reads from an accepted stream,
  clean end translated to `StreamInput.End`, argument-carrying spawned
  handler tasks over ordinary event, state, adapter context, routing metadata,
  and two additional ordinary metadata values using `task::spawn_with6`,
  plus a seven-argument variant with one more ordinary metadata value using
  `task::spawn_with7`, and an eight-argument variant with one additional
  ordinary metadata value using `task::spawn_with8`, plus a nine-argument
  variant with one additional ordinary metadata value using `task::spawn_with9`,
  plus a ten-argument variant with one additional ordinary metadata value using
  `task::spawn_with10`, plus an eleven-argument variant with one additional
  ordinary metadata value using `task::spawn_with11`, plus a twelve-argument
  variant with one additional ordinary metadata value using
  `task::spawn_with12`, plus a thirteen-argument variant with one additional
  ordinary metadata value using `task::spawn_with13`, plus a fourteen-argument
  variant with one additional ordinary metadata value using
  `task::spawn_with14`, plus a fifteen-argument variant with one additional
  ordinary metadata value using `task::spawn_with15`, plus a sixteen-argument
  variant with one additional ordinary metadata value using
  `task::spawn_with16`, plus a seventeen-argument variant with one additional
  ordinary metadata value using `task::spawn_with17`, plus an
  eighteen-argument variant with one additional ordinary metadata value using
  `task::spawn_with18`, plus a nineteen-argument variant with one additional
  ordinary metadata value using `task::spawn_with19`, plus a twenty-argument
  variant with one additional ordinary metadata value using
  `task::spawn_with20`, plus a twenty-one-argument variant with one
  additional ordinary metadata value using `task::spawn_with21`, plus a
  twenty-two-argument variant with one additional ordinary metadata value
  using `task::spawn_with22`, plus a twenty-three-argument variant with one
  additional ordinary metadata value using `task::spawn_with23`, plus a
  twenty-four-argument variant with one additional ordinary metadata value
  using `task::spawn_with24`, plus a twenty-five-argument variant with one
  additional ordinary metadata value using `task::spawn_with25`, plus a
  twenty-six-argument variant with one additional ordinary metadata value
  using `task::spawn_with26`, plus a twenty-seven-argument variant with one
  additional ordinary metadata value using `task::spawn_with27`, plus a
  twenty-eight-argument variant with one additional ordinary metadata value
  using `task::spawn_with28`, plus a twenty-nine-argument variant with one
  additional ordinary metadata value using `task::spawn_with29`, plus a
  thirty-argument variant with one additional ordinary metadata value using
  `task::spawn_with30`, plus a thirty-one-argument variant with one
  additional ordinary metadata value using `task::spawn_with31`, plus
  thirty-two-argument, thirty-three-argument, thirty-four-argument,
  thirty-five-argument, thirty-six-argument, thirty-seven-argument,
  thirty-eight-argument, and thirty-nine-argument variants with one additional
  ordinary metadata value each using
  `task::spawn_with32`, `task::spawn_with33`, `task::spawn_with34`,
  `task::spawn_with35`, `task::spawn_with36`, `task::spawn_with37`, and
  `task::spawn_with38`, and `task::spawn_with39`,
  deadline-aware accepted stream reads that stop on
  `net::read_chunk_until` returning `None`, cancellable accepted-stream
  routing that turns `WaitCancelled` into an ordinary cleanup action, and
  ordered write projection. Dedicated close-lifecycle examples additionally
  call `net::close_stream` after ordered writes or cancellation cleanup; they
  add no new effect label or compiler-known routing call. The owned-lifecycle
  and close-lifecycle adapters declare `net` and `concurrency`; the
  deadline-aware, cancellable, and cancel-close lifecycle adapters declare
  `net`, `time`, and `concurrency`; the pure handler boundary remains free of
  transport effects.
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
- Receiver-list channel-first routing effect coverage extends through the
  thirty-route checked example: channel selection carries
  `concurrency`, while the selected stream handler remains effect-free.
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
