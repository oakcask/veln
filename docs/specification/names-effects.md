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
  and stream calls, relative deadline calls, and cancellable deadline waits
  through source-visible `CancelToken` handles. Malformed receive fixtures,
  failed send or write recording, forced read or write failures, forced
  timeout or deadline expiry, and forced cancellable-wait cancellation are
  runtime failures.
  The socket stream adapter routing example composes existing `net` stream
  calls with existing channel and task calls under `concurrency`, including
  multiple fixture-backed reads from one stream; it adds no new effect label
  or compiler-known routing call.
- Prelude helper signatures, value semantics, source-backed helper set, and
  descriptor-only helper boundary:
  [names-effects-full.md](names-effects-full.md#prelude-helpers).
- Source-visible `StreamInput`, `DecodeStep<T>`, `DecodeReadiness`,
  `DecodeError`, `EncodeStep<TState>`, and `EncodeError` ADTs plus pure byte
  vocabulary helpers for `Byte`, `ByteChunk`, `ByteView`, `ByteCount`,
  `ByteOffset`, compact hex fixture decoding, bounded `ByteView` slicing,
  outgoing `List<ByteChunk>` construction, fixed-width unsigned big-endian and
  little-endian reads and writes, fixed U8 field checks, and the narrow schema
  width-sample decode helper. The implemented pending-input slice
  appends incoming `StreamInput.Chunk` bytes into immutable retained
  `ByteChunk` values, bounds retention with `ByteCount`, uses `ByteView`
  prefixes for parsing, tracks absolute `ByteOffset` separately, and collects
  outgoing immutable chunks in `List<ByteChunk>` action values. Generated
  binary schema decode helpers
  return schema-local
  `Int` record fields unless the eligible single structural `map to Target`
  slice resolves a mapped `Int` record shape; generated decode-step helpers
  expose the same value shape through `DecodeStep<T>` for open input.
  Generated binary schema encode helpers for the exact-width primitive,
  supported reserved-bit, length-bounded `ByteView`, closed dispatch,
  extension dispatch, and same-module or imported public nested dispatch
  payload slices accept schema-local visible fields, using `ByteView` fields
  for length-bounded payloads and `SchemaDispatchPayload<T>` for extension
  dispatch payload fields, and return `Result<ByteChunk, EncodeError>`.
  `UInt16le`, `UInt24le`, and `UInt32le` fields use little-endian byte order
  in generated decode and encode helpers.
  HTTP/2 frame-header decoding,
  bounded payload frame decoding, and protocol diagnostic projection helpers
  including stream id domain and post-GOAWAY stream failure projection are
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
