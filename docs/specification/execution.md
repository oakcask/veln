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
  Source-visible `ByteView` range failures expose command-facing
  `codec.byte_range_out_of_bounds` diagnostics with requested offset, requested
  count, available count, and bounded byte preview. Checked byte write
  conversion overflow exposes command-facing
  `codec.byte_write_value_unrepresentable` value diagnostics with helper name,
  supplied value, accepted range, width, and byte order.
  HTTP/2 SETTINGS value peer-limit diagnostics keep the failed range fact in
  the primary message, carry byte offset, setting identity, observed value,
  accepted range, and peer-limit provenance as separate structured fields, and
  render the offending SETTINGS item `ByteView` as a related bounded byte
  preview.
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
- `net` and `time` calls are host runtime boundaries:
  descriptor chunk receive/send, listener creation, accept, optional
  clean-end listener accept, deadline-aware optional listener accept, stream
  read, optional clean-end stream read, deadline-aware optional stream read,
  cancellable deadline-aware listener accept, cancellable deadline-aware
  stream read, stream write, ordered stream chunk-list write, stream close,
  listener close, monotonic clock reads, timeout, deadline waits, and
  cancellable deadline waits execute outside the
  pure protocol core. The
  default socket path is fixture-backed. With `VELN_NET_RUNTIME` set to
  `production-loopback`, the same public listen, accept, read, write, and
  close calls own a host loopback listener and deterministic accepted stream
  sequence; optional and deadline-aware accept can observe clean listener end
  after the planned loopback streams are exhausted, explicit listener close
  releases the listener without closing already accepted streams, and
  deadline-aware reads observe clean stream end through the same `None` result
  as the fixture path.
  `CancelToken` handles are source-visible time-boundary values used by
  adapter-owned waits. `CancelOwner` values let adapter code keep
  cancellation authority while exposing observer `CancelToken` handles to
  wait, channel, and socket code. Owner-derived observer tokens cannot be
  cancelled through direct `time::cancel(token)`; direct tokens created by
  `time::cancel_token` keep that compatibility path. `time::is_cancelled`
  observes whether such a token has already been cancelled without waiting or
  requesting cancellation.
  `CancellableWaitOutcome` values let adapter-owned waits observe completion,
  deadline expiry, or cancellation without stopping the entry. Stream adapter
  examples compose those outcomes with channel-routed `StreamInput` values,
  receiver-list channel-first selection, accepted socket streams, cancellable
  deadline-aware accepted streams, and ordinary response actions in fixture
  output so completed waits, deadline expiry, and cancellation become adapter
  routing decisions.
  `net::write_chunks` writes a `List<ByteChunk>` to the selected stream in
  source list order through the same stream write path as `net::write_chunk`.
  `net::write_chunks_until` writes the same list shape in source order while
  observing a `Deadline`, returning `WriteCompleted` only when the full list
  is written and returning `WriteDeadlineExpired` when deadline expiry wins
  before the list is fully written. `net::write_chunks_until_cancellable`
  extends that boundary with a `CancelToken` and also returns
  `WriteCancelled` when cancellation wins before the list is fully written.
  The checked adapter-owned outbound ordering example is
  `examples/specification/run/socket-stream-adapter-write-chunks-ordering/`;
  it accepts deterministic loopback streams, routes ordinary inputs through a
  channel, calls multiple pure handlers, combines their response actions into
  one explicit outbound order, and writes only the ordered `SendBytes` chunks
  with `net::write_chunks`.
  The checked clean shutdown adapter example is
  `examples/specification/run/socket-stream-adapter-clean-shutdown/`; it
  accepts an owned stream, routes ordinary `StreamInput` values through a
  channel, observes cancellation and deadline-expiry decisions as ordinary
  source values, projects only `SendBytes` actions to `net::write_chunk`, and
  then records `net::close_stream` followed by `net::close_listener`.
  The checked cancellation-owner adapter example is
  `examples/specification/run/socket-stream-adapter-cancel-owner-lifecycle/`;
  it keeps the `CancelOwner` in adapter cleanup, passes only the observer
  `CancelToken` to routing and socket code, requests cancellation through the
  owner, then observes cancelled wait and read attempts as ordinary outcome
  values before closing the owned transport handles. The checked
  `examples/specification/run/transport-cancel-owner-observer-only-json/`
  case keeps direct cancellation of an owner-derived observer token on the
  runtime-failure surface.
  The checked `examples/specification/run/transport-monotonic-clock/` case
  observes two `time::monotonic_ms` values and checks only monotonic ordering;
  the runtime exposes no wall-clock timestamp, date, time zone, sleep handle,
  or calendar-time conversion.
  Executable fixtures can set `VELN_TIME_CANCELLABLE_OUTCOMES` to
  a comma-separated sequence of `completed`, `deadline-expired`, and
  `cancelled` values for the value-returning wait path.
  Malformed received or read bytes, failed outgoing send, write, stream close,
  or listener close event recording, and forced listen, accept, read, write,
  close, timeout, deadline, or cancellable-wait cancellation failures through
  the runtime-failure wait stop the entry as runtime failures rather than
  schema, codec, or peer protocol diagnostics. After `net::close_listener`,
  `net::accept`, `net::accept_or_end`, `net::accept_until`, and
  `net::accept_until_cancellable` fail through that runtime boundary.
  `net::accept_until` turns accept
  deadline expiry into `None`, and `net::read_chunk_until` turns read deadline
  expiry into `None`, while forced host accept or read failure through those
  paths remains a runtime failure. `net::accept_until_cancellable` returns
  ordinary `AcceptOutcome` values for accepted stream, clean listener end,
  accept deadline expiry, and token cancellation while preserving forced host
  accept failures as runtime failures. `net::read_chunk_until_cancellable`
  returns ordinary `StreamReadOutcome` values for chunk arrival, clean end,
  read deadline expiry, and token cancellation while preserving forced host
  read failures as runtime failures. `net::write_chunk_until` returns
  ordinary `StreamWriteOutcome` values for completed write and write deadline
  expiry while preserving forced host write failures as runtime failures.
  `net::write_chunk_until_cancellable` returns ordinary `StreamWriteOutcome`
  values for completed write, write deadline expiry, and token cancellation
  while preserving forced host write failures as runtime failures.
  `net::write_chunks_until` and `net::write_chunks_until_cancellable` apply
  the same outcome and failure boundary to source-owned chunk lists.
- Stream adapter event-boundary examples use ordinary source ADT, record, and
  list values for decoded stream events and response actions. A handler
  receives an event plus explicit state and returns action intent values plus
  the next state. Channel routing uses existing `concurrency` calls; response
  actions do not perform socket writes or introduce new effect labels.
- The socket stream adapter routing context example composes ordinary event,
  state, route, and trace metadata into one anonymous record passed through
  `task::spawn_with<Result, Context>`. The spawned handler receives that one
  context parameter, returns ordinary action intent values plus next state,
  and stays free of socket handles and `net` calls. The checked example is
  `examples/specification/run/socket-stream-adapter-routing-context/`.
  Socket lifecycle examples still cover accepted-stream ownership, explicit
  close after clean end or cancellation cleanup, deadline-aware accepted-stream
  ownership, cancellation-to-action routing for an accepted stream, and the
  cancellable deadline-aware accept/read lifecycle boundary. The checked
  production loopback lifecycles are
  `examples/specification/run/socket-stream-adapter-production-lifecycle/`
  for one adapter-owned stream,
  `examples/specification/run/socket-stream-adapter-production-two-streams/`
  for two independent adapter-owned streams accepted from one listener through
  the same ordinary handler/action boundary with ordered writes, explicit
  closes, captured client bytes, and clean listener end,
  `examples/specification/run/socket-stream-adapter-production-drain-lifecycle/`
  for a listener-draining adapter that recursively accepts configured
  production streams until clean listener end while reusing the same ordinary
  handler/action boundary for every accepted stream,
  `examples/specification/run/socket-stream-adapter-production-deadline-lifecycle/`
  for the same production handler/action boundary through deadline-aware
  accept and read calls, followed by explicit close and clean listener end,
  `examples/specification/run/socket-stream-adapter-production-cancellable-deadline-lifecycle/`
  and
  `examples/specification/run/socket-stream-adapter-production-cancellable-deadline-outcomes/`
  for the same production handler/action boundary through cancellable
  deadline-aware accept/read outcomes, explicit stream close, clean listener
  end, and explicit listener close,
  `examples/specification/run/socket-stream-adapter-production-owner-drain-cancellable-deadline-lifecycle/`
  for a production listener-draining adapter that creates a `CancelOwner`,
  passes only observer `CancelToken` values to cancellable deadline-aware
  accept/read and channel-routing code, projects ordered `SendBytes` actions
  through `net::write_chunks`, and keeps cancellation authority in adapter
  cleanup while checking clean listener end and accept cancellation,
  `examples/specification/run/socket-stream-adapter-production-accept-until-failure-json/`
  and
  `examples/specification/run/socket-stream-adapter-production-read-until-failure-json/`
  for forced deadline-aware production accept and read failures,
  `examples/specification/run/socket-stream-adapter-production-drain-read-failure-json/`
  for a forced read failure after adapter-owned production accept but before
  response writes or stream close,
  `examples/specification/run/socket-stream-adapter-write-chunks-failure-json/`
  for a forced adapter-owned outbound write failure after production accept,
  read, ordinary handler routing, and ordered `SendBytes` projection through
  `net::write_chunks`,
  `examples/specification/run/socket-stream-adapter-production-close-failure-json/`
  for a forced close failure after an adapter-routed production stream has
  already projected ordered response writes,
  and `examples/specification/run/transport-socket-production-two-streams/`;
  production listen and close failures remain runtime failures and are checked
  by that close-failure case and
  `examples/specification/run/transport-socket-production-listen-failure-json/`.
- The channel-first stream routing examples route ordinary `StreamInput`
  values through typed channel routes, select the next ready route with the
  existing channel selection vocabulary, and only then invoke a plain handler
  with explicit per-stream state. The general receiver-list helper example
  accepts a non-empty `List<Receiver<StreamInput>>` and returns the selected
  route index plus value, so additional route-count fixtures are not required
  to demonstrate larger receiver lists. The receiver-list priority examples use
  `channel::select_many_priority` on a non-empty
  `List<Receiver<StreamInput>>`; the timeout example uses
  `channel::select_many_timeout` and
  `channel::select_many_timeout_result` to preserve supplied list order as
  priority order while returning `None` or `Ok(None)` when no receiver is
  ready before the timeout. The cancellable receiver-list timeout helper
  `channel::select_many_timeout_cancellable` uses the same priority, timeout,
  and selected value shape, and returns `Err(SelectError)` when its
  `CancelToken` is already cancelled or wins during the wait.
  The two-receiver timeout-result helper
  `channel::select_timeout_result` preserves the left/right receiver index
  shape, rotating tie behavior, timeout behavior, closed-before-selection
  `Ok(None)` behavior, `Int` timeout argument, and fallible selection boundary
  while requiring only `concurrency`.
  The two-receiver cancellable timeout helper
  `channel::select_timeout_cancellable` preserves the left/right receiver
  index shape, timeout behavior, and token-cancellation result boundary.
  When multiple receivers are ready, the earliest receiver in the supplied
  list wins. The handler remains an ordinary source function over stream input
  and state; adapter code owns channel routing, and socket wrappers around the
  same boundary own `NetStream` handles and writes. The primary checked
  examples are
  `examples/specification/run/channel-first-stream-routing-general-list/`,
  `examples/specification/run/channel-first-stream-routing/`,
  `examples/specification/run/channel-first-stream-routing-three-route/`,
  `examples/specification/run/channel-first-stream-routing-four-route/`,
  `examples/specification/run/channel-select-many-timeout/`,
  `examples/specification/run/channel-select-timeout-result/`,
  `examples/specification/run/channel-select-timeout-cancellable/`,
  `examples/specification/run/channel-select-many-timeout-cancellable/`,
  `examples/specification/run/channel-select-many-timeout-cancellable-forced-cancel/`,
  `examples/specification/run/stream-adapter-cancellable-channel-first-routing/`,
  `examples/specification/check/channel-first-stream-routing-effects/`,
  `examples/specification/check/channel-first-stream-routing-general-list-effects/`,
  `examples/specification/check/channel-first-stream-routing-three-route-effects/`,
  `examples/specification/check/channel-first-stream-routing-four-route-effects/`,
  `examples/specification/check/channel-select-many-timeout-effects/`,
  `examples/specification/check/channel-select-timeout-result-effects/`,
  `examples/specification/check/channel-select-timeout-cancellable-effects/`,
  `examples/specification/check/channel-select-many-timeout-cancellable-effects/`,
  and
  `examples/specification/check/stream-adapter-cancellable-channel-first-routing-effects/`.
  Earlier bounded route-count examples remain checked coverage, not a pattern
  for adding more same-shaped fixtures.
- The generated binary schema helper execution slice decodes the
  `Http2FrameHeaderWire` field sequence from a `ByteView`: `UInt24be`,
  `UInt8`, `UInt8`, `ReservedBits(1, 0)`, and `UInt31be`. The decoded value
  exposes ordinary `Int` fields for `length`, `kind`, `flags`, and
  `stream_id`. The reserved field is consumed and validated but is not
  exposed in the mapped record. Truncated schema fields report
  `schema.truncated_field`; invalid reserved bits report
  `schema.reserved_bits_mismatch`. Both carry byte offset and schema field
  path details. The checked examples are
  `examples/specification/run/binary-schema-frame-header-decode/`,
  `examples/specification/run/binary-schema-frame-header-truncated-json/`,
  `examples/specification/run/binary-schema-frame-header-truncated-human/`,
  `examples/specification/run/binary-schema-frame-header-reserved-json/`, and
  `examples/specification/run/binary-schema-frame-header-reserved-human/`.
- The `SchemaWidthSample` primitive decode slice consumes `UInt16be` followed
  by `UInt32be` from a `ByteView`. Both visible fields decode to ordinary
  `Int` values. Truncated fields use the same `schema.truncated_field` byte
  diagnostic shape as the frame-header slice, including byte offset, field
  path, expected count, available count, readiness, and structured byte
  preview fields.
- Generated binary schema decode helpers also support `UInt16le`, `UInt24le`,
  `UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le`
  as little-endian unsigned primitives.
  `UInt40be` is accepted as the matching big-endian five-byte primitive,
  `UInt48be` as the matching big-endian six-byte primitive, `UInt56be` as
  the matching big-endian seven-byte primitive, and `UInt64be` as the
  matching big-endian eight-byte primitive.
  These forms decode to ordinary `Int` fields for values representable as
  source-visible `Int`, preserve structural `map to` runtime mappings, and
  use the same truncation diagnostic shape as the other exact-width
  primitives.
  The checked examples are
  `examples/specification/run/binary-schema-u40-widths-decode/`,
  `examples/specification/run/binary-schema-u40-widths-truncated-json/`,
  `examples/specification/run/binary-schema-u48-widths-decode/`,
  `examples/specification/run/binary-schema-u56-widths-decode/`,
  `examples/specification/run/binary-schema-u56-widths-truncated-json/`,
  `examples/specification/run/binary-schema-u64-widths-decode/` and
  `examples/specification/run/binary-schema-u64-widths-truncated-json/`.
- Generated binary schema decode helpers support standalone visible `UInt1`
  through `UInt7` fields. Each field consumes one byte, exposes the declared
  low bits as an ordinary `Int`, advances by one byte, preserves structural
  `map to` runtime mappings, and uses the same `schema.truncated_field`
  diagnostic shape as other exact-width primitives. The checked examples are
  `examples/specification/run/binary-schema-sub-byte-decode/`,
  `examples/specification/run/binary-schema-sub-byte-decode-human/`,
  `examples/specification/run/binary-schema-sub-byte-truncated-json/`, and
  `examples/specification/run/binary-schema-sub-byte-truncated-human/`.
- Generated binary schema decode helpers also support consecutive
  visible-only `UInt1` through `UInt7` fields when at least two fields are
  present and their widths complete exactly one byte or one two-byte
  big-endian storage unit. The first field occupies the high bits, later
  fields occupy progressively lower bits, each decoded field remains an
  ordinary visible `Int`, the helper advances by the shared storage unit, and
  truncation reports `schema.truncated_field` at the first field in the group.
  Standalone or incomplete sub-byte fields keep the standalone
  one-byte-per-field behavior above. The checked examples are
  `examples/specification/run/binary-schema-packed-visible-byte-decode-encode/`
  and
  `examples/specification/run/binary-schema-packed-visible-byte-truncated-json/`
  for one byte, plus
  `examples/specification/run/binary-schema-packed-visible-two-byte-decode-encode/`,
  `examples/specification/run/binary-schema-packed-visible-two-byte-truncated-json/`,
  `examples/specification/run/binary-schema-packed-visible-two-byte-encode-out-of-range/`,
  and
  `examples/specification/run/derived-codec-packed-visible-two-byte-boundary/`
  for two bytes.
- Generated binary schema decode helpers support opt-in `Flag8`,
  `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`, `Flag32be`, `Flag32le`,
  `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`, `Flag56le`,
  `Flag64be`, and `Flag64le` fields as visible flag bitsets. They consume the
  same byte width, byte order, and truncation
  behavior as `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`,
  `UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`,
  `UInt56be`, `UInt56le`, `UInt64be`, and `UInt64le`, but the decoded record
  fields are source-visible
  `Flag8(bits)`, `Flag16be(bits)`, `Flag16le(bits)`, `Flag24be(bits)`,
  `Flag24le(bits)`, `Flag32be(bits)`, `Flag32le(bits)`,
  `Flag40be(bits)`, `Flag40le(bits)`, `Flag48be(bits)`, `Flag48le(bits)`,
  `Flag56be(bits)`, `Flag56le(bits)`, `Flag64be(bits)`, and `Flag64le(bits)`
  values rather than raw `Int` values.
  Existing `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`,
  `UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`,
  `UInt56be`, `UInt56le`, `UInt64be`, and `UInt64le` declarations continue
  to decode as ordinary `Int` fields. Pure prelude helpers inspect or set
  `Flag8` bit indexes `0` through `7`, `Flag16be` and `Flag16le` bit indexes
  `0` through `15`, `Flag24be` and `Flag24le` bit indexes `0` through `23`,
  `Flag32be` and `Flag32le` bit indexes `0` through `31`, `Flag40be` and
  `Flag40le` bit indexes `0` through `39`, `Flag48be` and `Flag48le` bit
  indexes `0` through `47`, `Flag56be` and `Flag56le` bit indexes `0` through
  `55`, and `Flag64be` and `Flag64le` bit indexes `0` through `63`, returning
  `Result` failures for indexes outside each helper's range. Raw-bit helpers
  expose decoded `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`,
  `Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
  `Flag56be`, `Flag56le`, `Flag64be`, and `Flag64le` integer bits and
  construct flag values for encode only when the supplied integer fits the
  matching flag width.
  The checked examples are
  `examples/specification/run/binary-schema-flag8-decode/`,
  `examples/specification/run/binary-schema-flag16be-decode/`,
  `examples/specification/run/binary-schema-flag8-bit-helpers/`,
  `examples/specification/run/binary-schema-flag8-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag8-bit-index-json/`,
  `examples/specification/run/binary-schema-flag8-bit-index-human/`,
  `examples/specification/run/binary-schema-flag16be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag16be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag16be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag16be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag16le-decode/`,
  `examples/specification/run/binary-schema-flag16le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag16le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag16le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag16le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag24-decode/`,
  `examples/specification/run/binary-schema-flag24-bit-helpers/`,
  `examples/specification/run/binary-schema-flag24-helper-diagnostics-json/`,
  `examples/specification/run/binary-schema-flag32be-decode/`,
  `examples/specification/run/binary-schema-flag32be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag32be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag32be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag32be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag32le-decode/`,
  `examples/specification/run/binary-schema-flag32le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag32le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag32le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag32le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag40be-decode/`,
  `examples/specification/run/binary-schema-flag40be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag40be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag40be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag40be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag40le-decode/`,
  `examples/specification/run/binary-schema-flag40le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag40le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag40le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag40le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag48be-decode/`,
  `examples/specification/run/binary-schema-flag48be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag48be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag48be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag48be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag48le-decode/`,
  `examples/specification/run/binary-schema-flag48le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag48le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag48le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag48le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag56be-decode/`,
  `examples/specification/run/binary-schema-flag56be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag56be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag56be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag56be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag56le-decode/`,
  `examples/specification/run/binary-schema-flag56le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag56le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag56le-bit-index-json/`,
  `examples/specification/run/binary-schema-flag56le-bit-index-human/`,
  `examples/specification/run/binary-schema-flag64be-decode/`,
  `examples/specification/run/binary-schema-flag64be-bit-helpers/`,
  `examples/specification/run/binary-schema-flag64be-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag64be-bit-index-json/`,
  `examples/specification/run/binary-schema-flag64be-bit-index-human/`,
  `examples/specification/run/binary-schema-flag64le-decode/`,
  `examples/specification/run/binary-schema-flag64le-bit-helpers/`,
  `examples/specification/run/binary-schema-flag64le-from-bits-out-of-range-json/`,
  `examples/specification/run/binary-schema-flag64le-bit-index-json/`, and
  `examples/specification/run/binary-schema-flag64le-bit-index-human/`.
- Generated binary schema decode helpers support bounded
  `Repeat(count_field, Payload)` fields when `count_field` is an earlier
  visible exact-width unsigned field decoded as `Int` and `Payload` is
  `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`,
  `UInt31le`, `UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`,
  `UInt48le`, `UInt56be`, `UInt56le`, `UInt64be`, `UInt64le`, an eligible
  same-module or public imported nested binary schema payload, or
  `ByteView(length_field)` when
  `length_field` is another earlier visible exact-width unsigned field decoded
  as `Int`.
  `Repeat(left_count - right_count, Payload)`,
  `Repeat(left_count + right_count, Payload)`, and
  `Repeat(left_count * right_count, Payload)`, and
  `Repeat(left_count / right_count, Payload)` use the difference, sum,
  product, or integer quotient of two earlier visible exact-width unsigned
  `Int` fields as the repeat count. A
  repeated primitive field decodes to `List<Int>`; a repeated
  nested schema field decodes to a list of the nested schema's decoded record
  shape, including when the schema is named through a written `use` path; and
  a repeated `ByteView(length_field)` field decodes to
  `List<ByteView>` with each element preserving its bounded bytes in element
  order. The helper reads exactly the computed count in declaration order. A
  negative computed count reports `schema.length_out_of_bounds` at the repeat
  field path. Division by zero reports `schema.length_division_by_zero` at the
  repeat field path. Truncation is reported at the first element that cannot
  be fully read with the usual `schema.truncated_field` details and a schema
  field path that appends an `index` segment before nested schema field
  segments. The
  checked examples are `examples/specification/run/binary-schema-repeat-decode/`,
  `examples/specification/run/binary-schema-repeat-add-decode/`,
  `examples/specification/run/binary-schema-repeat-subtract-decode/`,
  `examples/specification/run/binary-schema-repeat-product-decode/`,
  `examples/specification/run/binary-schema-repeat-quotient-decode/`,
  `examples/specification/run/binary-schema-repeat-subtract-negative-json/`,
  `examples/specification/run/binary-schema-repeat-product-negative-json/`,
  `examples/specification/run/binary-schema-repeat-quotient-division-by-zero-json/`,
  `examples/specification/run/binary-schema-repeat-truncated-json/`,
  `examples/specification/run/binary-schema-repeat-truncated-human/`,
  `examples/specification/run/binary-schema-repeat-nested-decode/`,
  `examples/specification/run/binary-schema-imported-repeat-nested-decode/`,
  `examples/specification/run/binary-schema-repeat-nested-truncated-json/`,
  `examples/specification/run/binary-schema-imported-repeat-nested-truncated-json/`,
  `examples/specification/run/binary-schema-repeat-byteview-decode/`, and
  `examples/specification/run/binary-schema-repeat-byteview-truncated-json/`.
- Generated binary schema decode helpers support byte-aligned
  `ReservedBits(width, value)` fields up to four bytes wide as
  representation-only fields. The helper consumes the reserved bytes in
  declaration order, validates the declared fixed value, omits the field from
  the decoded value and structural mapping source values, and reports
  `schema.truncated_field` or `schema.reserved_bits_mismatch` at the reserved
  field path when the input is short or the fixed value differs.
- Generated binary schema decode helpers also support packed reserved
  prefixes: `ReservedBits(width, value)` where `width` is one through seven
  may be followed by the visible `UIntN` primitive whose width completes the
  byte, widths nine through fifteen may be followed by the visible `UIntN`
  primitive whose width completes the same two-byte big-endian storage unit,
  and widths seventeen through twenty-three may be followed by the visible
  `UIntN` primitive whose width completes the same three-byte big-endian
  storage unit, and widths twenty-five through thirty-one may be followed by
  the visible `UIntN` primitive whose width completes the same four-byte
  big-endian storage unit. The helper validates the high reserved bits,
  decodes the low
  visible bits as an ordinary `Int`, omits the reserved field from decoded
  records and mapping source values, and advances by the shared storage width
  for the pair. The width-fifteen two-byte boundary supports
  `ReservedBits(15, value)` followed immediately by visible `UInt1`; it uses
  the same reserved-bit mismatch shape, mapping-source omission, derived
  codec eligibility, and encode range-failure path. Checked examples are
  `examples/specification/run/binary-schema-reserved-fifteen-bit-prefix-decode-encode/`
  and
  `examples/specification/run/binary-schema-reserved-fifteen-bit-prefix-json/`.
  The inverse suffix layout is also supported: a visible
  `UIntN` field followed immediately by `ReservedBits(width, value)` where
  the two widths complete one byte or the same two-byte, three-byte, or
  four-byte big-endian storage unit, plus the five-byte case where the fields
  complete forty bits and the six-byte case where the fields complete
  forty-eight bits. That form decodes the visible value from the high bits,
  validates the low reserved bits at the reserved field path, omits the
  reserved field, and
  advances by the shared storage width. The supported middle layout is a
  visible `UIntN` field, a `ReservedBits(width, value)` field, and another
  visible `UIntN` field whose widths together complete one byte or the same
  two-byte, three-byte, or four-byte big-endian storage unit. That form
  decodes the visible fields from their declared high-to-low positions,
  validates the middle reserved field at the reserved field path, omits the
  reserved field, and advances by the shared storage width. The narrow
  two-byte interleaved form also accepts a sub-byte visible `UIntN` field, a
  sub-byte middle `ReservedBits(width, value)` field, a byte-width visible
  `UInt8` field, and a final sub-byte visible `UIntN` field when the four
  widths complete the same two-byte big-endian storage unit without completing
  a storage byte before the `UInt8` field. A supported
  prefix group may also place `ReservedBits(width, value)` before two visible
  sub-byte or byte-width `UIntN` fields when all three widths complete one
  byte, a two-byte big-endian storage unit, a three-byte big-endian
  storage unit, a four-byte big-endian storage unit, a five-byte
  big-endian storage unit, a six-byte big-endian storage unit, a seven-byte
  big-endian storage unit, or an eight-byte big-endian storage unit. In the
  two-byte form,
  reserved prefix widths one through
  fourteen are accepted when the two visible fields complete the remaining
  bits in declaration order; in the three-byte form, reserved prefix widths
  seventeen through twenty-three are accepted when the two visible fields
  complete the remaining bits in declaration order; in the four-byte form,
  reserved prefix widths twenty-five through thirty-one are accepted when the
  two visible fields complete the remaining bits in declaration order; in the
  five-byte form, a reserved prefix width thirty-three is accepted when the
  two visible fields complete the remaining bits in declaration order; in the
  six-byte form, a reserved prefix width forty-one is accepted when the
  two visible fields complete the remaining bits in declaration order; in the
  seven-byte form, a reserved prefix width forty-nine is accepted when the
  two visible fields complete the remaining bits in declaration order; in the
  eight-byte form, a reserved prefix width fifty-seven is accepted when the
  two visible fields complete the remaining bits in declaration order. That
  form validates the high reserved bits, decodes the following visible fields
  from their declared high-to-low positions, omits the reserved field, and
  advances by the shared storage width. A narrow two-byte suffix group may
  place two visible `UIntN` fields before a non-byte-aligned
  `ReservedBits(width, value)` suffix when the second visible field is
  `UInt8` and all three widths complete the same two-byte big-endian storage
  unit. That
  form decodes the two visible fields from their declared high-to-low
  positions, validates the low reserved bits, omits the reserved field, and
  advances by two bytes. The narrow `ReservedBits(2, 0)` and
  `ReservedBits(9, 0)` prefixes followed by `UInt8` also use a two-byte
  big-endian bitstream slice: the reserved prefix is validated first, the
  visible byte is decoded from the following byte position, trailing low
  padding bits are ignored when present, and the reserved field is omitted
  from decoded records and mapping source values. The same
  shared-storage rule also covers
  consecutive non-byte-aligned `UIntN` and `ReservedBits(width, value)`
  fields when the group contains at least one visible field and at least one
  reserved field, every visible field is a big-endian sub-byte `UIntN`, and
  the declared widths complete one byte or the same two-byte, three-byte,
  four-byte, five-byte, six-byte, seven-byte, or eight-byte big-endian storage
  unit. Reserved fields in the group remain representation-only, each
  reserved value is validated at its own field path, and visible fields are
  decoded from their declared high-to-low positions.
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
  value matches. A mismatch returns
  `Err(RuntimeDiagnostic(id, message, RuntimeByteDiagnostic(...)))` and
  reports `schema.fixed_field_mismatch` at the field byte offset with schema
  field path, expected value, actual value, and structured byte preview
  fields. The rendered `RuntimeDiagnostic(...)` is the result value projected
  by command output.
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
- The schema value validation slice exposes generated `validate_<schema>`
  helpers for eligible binary schema declarations. The helper accepts the
  schema-local decoded record shape, checks field-local `where` predicates in
  declaration order using the same supported predicate language as generated
  binary decode helpers, and returns the supplied record on success. A failed
  predicate reports `schema.validation_failed` with schema field path,
  predicate text, owning supplied field value, and supplied decoded `Int`
  values. Checked examples are
  `examples/specification/run/schema-value-validation/`,
  `examples/specification/run/schema-value-validation-json/`, and
  `examples/specification/run/schema-value-validation-human/`.
- The binary schema schema-level validation slice checks one `validate`
  predicate after all fields have decoded and after field-local validation has
  succeeded, but before structural mapping returns the decoded value.
  Predicates use the same supported expression subset as field-local `where`
  predicates and may reference only decoded `Int` fields from the same schema.
  Primitive, length, dispatch, repeat, reserved-bit, fixed-field, and
  field-local validation failures win before the schema-level predicate is
  evaluated. A failed predicate reports `schema.validation_failed` at the
  byte offset after the decoded schema body with schema path, predicate text,
  decoded values, and structured byte preview fields.
- The narrow binary schema closed dispatch slice decodes
  `Dispatch(tag_field, tag => Payload, ...)` fields after the referenced tag
  field has been decoded by an earlier exact-width field in the same schema.
  It also decodes `Dispatch(tag_field, length_field, tag => Payload, ...)`
  when the length field is an earlier visible `Int` field and the selected
  payload is read from that bounded byte range. Known dispatch cases consume
  either the selected exact-width unsigned payload primitive and expose an
  ordinary `Int` field, or the selected same-module or imported public nested
  binary schema through the generated schema helper path and expose that
  schema's decoded record shape. Nested
  payload decode failures report the outer dispatch field path, nested schema
  field path, and absolute byte offset from the enclosing input, including
  failures from the nested
  helper's fixed-field, reserved-bit, endian, mapping, and primitive decoding
  behavior. Unknown tags in the closed dispatch report
  `schema.dispatch_unknown_tag` at the dispatch field byte offset with schema
  field path, decoded tag field, decoded tag value, expected tags, and
  structured byte preview fields. Same-module recursive closed-dispatch
  payload cases and public imported recursive payload schemas named through a
  written `use` path are eligible only in the length-bounded form when
  selected mappings cover every dispatch case and all mappings resolve to one
  record shape, with at least one non-recursive case as the base case. The
  recursive helper path decodes the nested payload from the bounded dispatch
  range before continuing with later fields and preserves the same outer
  dispatch plus nested schema field path on failures. Resolved nested payload
  schemas must expose both generated decode-step and encode helpers before a
  parent dispatch helper can use them; a payload schema that can decode but
  whose mapping assignment cannot be projected back to schema-local fields for
  generated encode is rejected at check time with `schema.dispatch_payload`.
  The checked
  examples are
  `examples/specification/run/binary-schema-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-recursive-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
  `examples/specification/run/binary-schema-dispatch-byteview-payload-decode/`,
  `examples/specification/run/binary-schema-dispatch-reserved-payload-roundtrip/`,
  `examples/specification/run/binary-schema-imported-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-imported-dispatch-byteview-payload-decode/`,
  `examples/specification/run/binary-schema-imported-recursive-dispatch-decode/`,
  `examples/specification/run/binary-schema-dispatch-nested-failure-json/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-failure-json/`,
  `examples/specification/run/binary-schema-imported-dispatch-nested-failure-json/`,
  `examples/specification/run/binary-schema-recursive-dispatch-failure-json/`,
  `examples/specification/run/binary-schema-imported-recursive-dispatch-failure-json/`,
  `examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
  and
  `examples/specification/run/binary-schema-closed-dispatch-unknown-human/`.
- Closed dispatch cases may decode to mixed primitive and nested record payload
  shapes only when selected `map to Target when tag_field == literal` clauses
  cover the dispatch tag cases with distinct literals and all selected
  mappings resolve to the same target record shape. Each selected branch
  type-checks `payload` as the payload shape chosen by that branch literal;
  other fields keep their schema-local decoded types. The helper still decodes
  the dispatch case from the already decoded tag value before applying the
  selected mapping. Extension dispatch, selectors other than the dispatch tag,
  and uncovered or duplicate case selectors remain outside this mixed-payload
  boundary. The checked examples are
  `examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-decode/`
  and
  `examples/specification/check/binary-schema-mixed-dispatch-selected-mapping-diagnostics/`.
- The narrow binary schema extension dispatch slice decodes
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields
  after both referenced fields have been decoded by earlier exact-width fields
  in the same schema. Known cases consume either the selected exact-width
  unsigned payload primitive or the selected same-module or imported public
  nested binary schema through the generated schema helper path from the
  bounded payload bytes selected by `length_field`, then expose it as
  `SchemaDispatchPayload::Known(value)`.
  Same-module recursive known payload cases and public imported recursive
  payload schemas named through a written `use` path are eligible in the
  length-bounded form when selected mappings cover every known case, all
  mappings resolve to one record shape, and at least one known case is
  non-recursive. Recursive known cases decode through the same generated
  schema helper path within the bounded payload range.
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
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-decode/`,
  `examples/specification/run/binary-schema-dispatch-byteview-payload-decode/`,
  `examples/specification/run/binary-schema-dispatch-reserved-payload-roundtrip/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-imported-dispatch-byteview-payload-decode/`,
  `examples/specification/run/binary-schema-recursive-extension-dispatch-decode/`,
  `examples/specification/run/binary-schema-imported-recursive-dispatch-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-unknown/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-unknown/`,
  and
  `examples/specification/run/binary-schema-extension-dispatch-length-human/`.
  `examples/specification/run/binary-schema-general-helper-roundtrip/`
  combines `Flag8`, bounded repeat, supported reserved bits,
  `ByteView(left_length - right_length)`, and nested extension dispatch in a
  non-HTTP schema and checks successful decode followed by encode.
- When an eligible generated binary schema decode helper has one structural
  `map to Target` clause, or multiple structural mapping clauses selected by
  `when field == literal`, `when field != literal`, or boolean selector
  expressions built from decoded schema-local `Int` fields, integer literals,
  `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, and `not`, or by direct
  selector calls to one pure same-module `Bool` converter function or one
  imported public pure `Bool` converter function through a written `use` path
  or alias, and each target resolves to the same decoded record shape whose
  mapped expressions match the target field types,
  the helper returns the selected mapped ordinary record shape instead of the
  schema-local field shape. Mapping selection reads the already decoded `Int`
  selector fields after field-local validation succeeds; selector clauses must
  not overlap for any concrete assignment of those fields, so at most one
  mapping is selected. Converter selectors are evaluated after field-local
  validation succeeds, using the same schema-local field and supported
  structural mapping argument rules as mapping converter calls. Mapping assignment
  expressions may reference decoded schema fields, construct records,
  construct ADT payloads resolved through the ordinary source module rules,
  including nested ADT constructor payload expressions whose leaves stay in the
  implemented schema-local expression vocabulary, or
  call one pure same-module converter function or one imported public pure
  converter function through a written `use` path or alias. They may also
  select a field from an already supported structural mapping expression after
  the source expression is available, when that source expression has a
  record-shaped type with the selected field. An `Int` target field may also
  use `+`, `-`, `*`, and `/` over decoded schema-local `Int` fields, integer
  literals, `Int`-returning converter calls, and nested supported mapping
  arithmetic expressions. A `Bool` target field may use `==`, `!=`, `<`,
  `<=`, `>`, and `>=` over those supported `Int` mapping operands, and may
  compose those supported comparisons with `and`, `or`, and `not`. Division by zero returns
  `schema.mapping_division_by_zero` at the offset after the decoded schema
  body with the schema and target-field path. Converter calls
  take one, two, three, four, or five arguments. Each argument is either one decoded
  schema-local field or an already implemented structural mapping expression
  made from decoded schema fields, records, ADT constructors, integer
  arithmetic mapping expressions, pure converter calls, and nested
  combinations of those forms. The
  returned value is then assigned to the target field.
  Mapping assignment targets must name target fields, and every target field
  must be assigned once before execution. The implemented mapped decoded field
  types are exact-width unsigned primitive fields, including standalone
  `UInt1` through `UInt7`, as `Int`; `Flag8` fields as `Flag8`;
  `Flag16be` fields as `Flag16be`; `Flag16le` fields as `Flag16le`;
  `Flag24be` fields as `Flag24be`; `Flag24le` fields as `Flag24le`;
  `Flag32be` fields as `Flag32be`; `Flag32le` fields as `Flag32le`;
  `Flag40be` fields as `Flag40be`; `Flag40le` fields as `Flag40le`;
  `Flag48be` fields as `Flag48be`; `Flag48le` fields as `Flag48le`;
  `Flag56be` fields as `Flag56be`; `Flag56le` fields as `Flag56le`;
  `Flag64be` fields as `Flag64be`; `Flag64le` fields as `Flag64le`;
  length-bounded
  `ByteView(length_field)`, `ByteView(left_length - right_length)`,
  `ByteView(left_length + right_length)`, and
  `ByteView(left_length * right_length)`, and
  `ByteView(left_length / right_length)` payload fields as `ByteView`; closed
  nested dispatch payload fields as the nested schema record shape; closed
  mixed dispatch payload fields as the
  selected primitive `Int` or nested schema record shape inside the matching
  selector branch; and extension dispatch payload fields as
  `SchemaDispatchPayload<T>`. Mapping expressions cannot call bare imported
  converter names, private imported converter functions, arbitrary ordinary
  functions, read runtime settings, inspect stream state, recover from decode
  failures, or perform effects. The checked examples are
  `examples/specification/run/binary-schema-sub-byte-decode/`,
  `examples/specification/run/binary-schema-flag8-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag16be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag16le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag24-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag32be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag32le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag40be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag40le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag48be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag48le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag56be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag56le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag64be-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag64le-mapped-record-decode/`,
  `examples/specification/run/binary-schema-flag8-mapped-constructor-decode/`,
  `examples/specification/run/binary-schema-flag8-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-flag8-imported-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-mapped-record-decode/`,
  `examples/specification/run/binary-schema-mapped-byteview-decode/`,
  `examples/specification/run/binary-schema-mapped-record-expression-decode/`,
  `examples/specification/run/binary-schema-mapped-constructor-expression-decode/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-decode/`,
  `examples/specification/run/binary-schema-mapping-arithmetic-decode/`,
  `examples/specification/run/binary-schema-mapping-bool-comparison-decode/`,
  `examples/specification/run/binary-schema-mapping-bool-composition-decode/`,
  `examples/specification/run/binary-schema-mapping-ordered-comparison-decode/`,
  `examples/specification/run/binary-schema-mapping-converter-arithmetic-decode/`,
  `examples/specification/run/binary-schema-imported-mapping-converter-arithmetic-decode/`,
  `examples/specification/run/binary-schema-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-mapped-converter-adt-argument-decode/`,
  `examples/specification/run/binary-schema-nested-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-two-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-three-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-four-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-five-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-nested-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-mapped-converter-structural-argument-decode/`,
  `examples/specification/run/binary-schema-imported-two-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-three-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-four-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-imported-five-argument-mapped-converter-decode/`,
  `examples/specification/run/binary-schema-mapping-converter-selector-decode/`,
  `examples/specification/run/binary-schema-nested-mapping-converter-selector-decode/`,
  `examples/specification/run/binary-schema-imported-mapping-converter-selector-decode/`,
  `examples/specification/run/binary-schema-mapping-selection-decode/`,
  `examples/specification/run/binary-schema-mapping-selection-not-equal-decode/`,
  `examples/specification/run/binary-schema-mapping-ordered-selection-decode/`,
  `examples/specification/run/binary-schema-mapped-field-selection-decode/`,
  `examples/specification/run/binary-schema-mapped-constructor-field-selection-decode/`,
  `examples/specification/run/binary-schema-mapped-nested-dispatch-decode/`,
  and
  `examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-decode/`.
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
  visible exact-width unsigned primitive fields as ordinary `Int` values and
  whose `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`,
  `Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
  `Flag56be`, `Flag56le`, `Flag64be`, and `Flag64le` fields are
  source-visible `Flag8(bits)`, `Flag16be(bits)`, `Flag16le(bits)`,
  `Flag24be(bits)`, `Flag24le(bits)`, `Flag32be(bits)`, `Flag32le(bits)`,
  `Flag40be(bits)`, `Flag40le(bits)`, `Flag48be(bits)`, `Flag48le(bits)`,
  `Flag56be(bits)`, `Flag56le(bits)`, `Flag64be(bits)`, and
  `Flag64le(bits)` values. For one
  structural `map to Target` clause whose assignments project every visible
  encode field, the helper accepts the mapping target record shape instead
  and projects those target fields back to the schema-local encode record.
  The narrow inverse projection supports direct schema-local field
  references, record expressions whose fields are direct schema-local visible
  field references, field selection from those record expressions when the
  selected field maps directly to one schema-local visible field, and one
  target field assigned from a direct ADT constructor call whose payload
  arguments use those supported projectable field and record-expression forms
  already supported by the generated encode helper. Constructor payload
  arguments can themselves be nested ADT constructor calls when their leaves
  use those same projectable field and record-expression forms. For `Int`
  target fields, the inverse projection also supports the reversible
  arithmetic forms `target = field + literal`, `target = literal + field`,
  and `target = field - literal`; the helper recovers the schema-local field
  value and then writes through the same schema-local encode path, including
  primitive range checks and field-path diagnostics. A target
  field assigned from one pure same-module converter call or one imported
  public pure converter call through a written `use` path or alias is also
  projectable when the assignment names an explicit pure inverse converter
  through the same written path rules with `inverse name`; the helper calls
  the inverse, checks that applying the mapped converter to the projected
  value round-trips to the supplied target field value, then writes the
  recovered schema-local fields. Single-payload
  constructor wrappers remain limited to the existing single-constructor flag
  and exact-width integer cases unless the payload is that record-expression
  slice or a supported nested constructor projection. A target value whose
  ADT constructor does not match the constructor expected by the mapping
  returns
  `Err(EncodeError("codec.encode_mapping_mismatch", field_path, reason))`.
  If a nested constructor payload does not match its expected constructor or
  the expected record shape, the same `codec.encode_mapping_mismatch` id is
  returned at the mapped target field path. A converter inverse projection
  that does not round-trip through the mapped converter also returns
  `codec.encode_mapping_mismatch` at the mapped target field path. These
  mapped encode paths write bytes through the schema-local fields. The checked
  examples are
  `examples/specification/run/binary-schema-mapped-record-expression-encode/`
  and
  `examples/specification/run/binary-schema-mapped-field-selection-encode/`,
  `examples/specification/run/binary-schema-mapping-arithmetic-encode/`,
  `examples/specification/run/binary-schema-mapping-arithmetic-encode-out-of-range/`,
  `examples/specification/run/binary-schema-mapped-converter-encode/`, and
  `examples/specification/run/binary-schema-mapped-converter-encode-mismatch/`,
  `examples/specification/run/binary-schema-imported-mapped-converter-encode/`,
  `examples/specification/run/binary-schema-imported-mapped-converter-encode-mismatch/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode-outer-mismatch-json/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode-inner-mismatch-json/`,
  and
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode-out-of-range/`.
  A
  length-bounded `ByteView(length_field)`,
  `ByteView(left_length - right_length)`,
  `ByteView(left_length + right_length)`, or
  `ByteView(left_length * right_length)`, or
  `ByteView(left_length / right_length)` payload field is a `ByteView` record
  field and emits exactly the bounded bytes from that view after the earlier
  visible length operand fields are written. Decode computes arithmetic
  lengths from the earlier decoded field values, rejects negative results as
  `schema.length_out_of_bounds`, reports `schema.length_division_by_zero` when
  a division length expression has divisor zero, and reports
  `schema.length_out_of_bounds` when the computed payload length exceeds the
  remaining bytes. If the supplied view
  count differs from the earlier length field or computed length expression,
  the helper returns
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))` without emitting partial output. Command-facing diagnostics for
  this schema-facing conversion boundary preserve the schema field path,
  expected count, actual `ByteView` count, length expression, byte offset,
  bounded byte preview, and count mismatch reason in human and JSON output.
  The checked examples are
  `examples/specification/run/binary-schema-byteview-quotient-encode/`,
  `examples/specification/run/binary-schema-byteview-quotient-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-byteview-encode-diagnostic-json/`
  and
  `examples/specification/run/binary-schema-byteview-encode-diagnostic-human/`.
  Bounded
  repeated primitive fields are `List<Int>` record fields, repeated nested
  schema fields are list fields whose element type is the same-module or
  public imported nested schema's decoded record shape, and repeated
  `ByteView(length_field)` fields are
  `List<ByteView>` record fields. They emit exactly the number of elements
  named by the earlier count field or by the computed difference, sum,
  product, or integer quotient of two earlier count operands. A list length
  mismatch, a primitive
  element outside the selected primitive range, a repeated byte-view element
  whose bounded byte count differs from the earlier length field, or a nested
  element
  representation failure returns
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`; repeated byte-view element failures append the element index to
  the repeated field path, and nested element failures prefix the nested schema
  field path with the repeated field and element index. `Flag8` emits one
  byte through the same representation path as `UInt8`, `Flag16be` emits
  two bytes through the same big-endian representation path as `UInt16be`,
  `Flag16le` emits two bytes through the same little-endian representation
  path as `UInt16le`, `Flag24be` emits three bytes through the same
  big-endian representation path as `UInt24be`, `Flag24le` emits three bytes
  through the same little-endian representation path as `UInt24le`,
  `Flag32be` emits four bytes through the same big-endian representation path
  as `UInt32be`, `Flag32le` emits four bytes through the same little-endian
  representation path as `UInt32le`,
  `Flag40be` emits five bytes through the same big-endian representation path
  as `UInt40be`, `Flag40le` emits five bytes through the same little-endian
  representation path as `UInt40le`,
  `Flag48be` emits six bytes through the same big-endian representation path
  as `UInt48be`, `Flag48le` emits six bytes through the same little-endian
  representation path as `UInt48le`,
  `Flag56be` emits seven bytes through the same big-endian representation path
  as `UInt56be`, `Flag56le` emits seven bytes through the same little-endian
  representation path as `UInt56le`,
  `Flag64be` emits eight bytes through the same big-endian representation
  path as `UInt64be`, and `Flag64le` emits eight bytes through the same
  little-endian representation path as `UInt64le`;
  `bits` values outside the selected flag width return
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`. A byte-aligned `ReservedBits(width, value)` field is
  representation-only: it is omitted from the record and the helper emits the
  declared fixed value in declaration order. A `ReservedBits(1, 0)` field
  immediately before a
  `UInt31be` field keeps the shared stream-identifier layout: it is omitted
  from the record and the helper emits the required zero high bit in the
  shared four-byte position. A packed `ReservedBits(width, value)` field
  followed by the visible `UIntN` primitive whose width completes the same
  one-byte, two-byte, three-byte, or four-byte big-endian storage unit is also
  representation-only: the helper emits the high reserved bits from the
  declared value and the low visible bits from the encoder input record.
  `ReservedBits(15, value)` followed by `UInt1` is the width-fifteen
  two-byte boundary for that encode rule. A
  visible `UIntN` field followed by a `ReservedBits(width, value)` suffix
  that completes the same one-byte, two-byte, three-byte, or four-byte
  big-endian storage unit, plus the five-byte case where the fields complete
  forty bits and the six-byte case where the fields complete forty-eight
  bits, is representation-only in the same way, but emits the visible
  value in the high bits and the declared reserved value in the low bits. A
  visible `UIntN` field, middle `ReservedBits(width, value)`
  field, and following visible `UIntN` field whose widths complete the same
  storage unit are also representation-only: the helper writes both visible
  values around the declared reserved value in declaration order and reports
  `codec.encode_value_unrepresentable` at the out-of-range visible field.
  The same middle encode rule includes the narrow two-byte interleaved layout
  where a sub-byte visible field and sub-byte middle reserved field are
  followed by `UInt8` and a final sub-byte visible field.
  A supported prefix group with `ReservedBits(width, value)` followed by two
  visible sub-byte or byte-width `UIntN` fields whose widths complete one
  byte, a two-byte big-endian storage unit, a three-byte big-endian
  storage unit, a four-byte big-endian storage unit, a five-byte
  big-endian storage unit, a six-byte big-endian storage unit, a seven-byte
  big-endian storage unit, or an eight-byte big-endian storage unit writes
  the declared reserved value first, then the two visible values in
  declaration order. The two-byte encode form accepts
  reserved
  prefix widths one through fourteen when the visible fields complete the
  remaining bits, the three-byte encode form accepts reserved prefix widths
  seventeen through twenty-three when the visible fields complete the
  remaining bits, the four-byte encode form accepts reserved prefix widths
  twenty-five through thirty-one when the visible fields complete the
  remaining bits, the five-byte encode form accepts reserved prefix width
  thirty-three when the visible fields complete the remaining bits, the
  six-byte encode form accepts reserved prefix width forty-one when the
  visible fields complete the remaining bits, the seven-byte encode form
  accepts reserved prefix width forty-nine when the visible fields complete
  the remaining bits, the eight-byte encode form accepts reserved prefix
  width fifty-seven when the visible fields complete the remaining bits,
  and reports `codec.encode_value_unrepresentable` at the
  out-of-range visible field. A narrow two-byte suffix group with two visible
  `UIntN` fields followed by a non-byte-aligned
  `ReservedBits(width, value)` suffix writes the visible values in
  declaration order followed by the declared low reserved bits, when the
  second visible field is `UInt8` and all three widths complete the same
  two-byte big-endian storage unit. The narrow `ReservedBits(2, 0)` and
  `ReservedBits(9, 0)` prefixes followed by `UInt8` emit a two-byte
  big-endian bitstream slice with the declared reserved prefix first, the
  visible byte after it, and zero low padding bits when present; the reserved
  field remains omitted from the encoder value record. The same
  shared-storage encode rule also covers
  consecutive
  non-byte-aligned `UIntN` and `ReservedBits(width, value)` fields when the
  group contains at least one visible field and at least one reserved field,
  every visible field is a big-endian sub-byte `UIntN`, and the declared
  widths complete one byte or the same two-byte, three-byte, four-byte,
  five-byte, six-byte, seven-byte, or eight-byte big-endian storage unit. The
  helper writes visible and reserved values in declaration order, omits
  reserved fields from the encoder value record, and reports
  `codec.encode_value_unrepresentable` at the out-of-range visible field.
  Consecutive visible-only `UInt1` through `UInt7` fields whose widths
  complete exactly one byte or one two-byte big-endian storage unit use the
  same declaration-order bit packing without any reserved fields: the first
  value occupies the high bits, later values occupy progressively lower bits,
  and out-of-range values report `codec.encode_value_unrepresentable` at the
  offending field path. The checked examples are
  `examples/specification/run/binary-schema-packed-visible-byte-decode-encode/`
  and
  `examples/specification/run/binary-schema-packed-visible-byte-encode-out-of-range/`
  for one byte, plus
  `examples/specification/run/binary-schema-packed-visible-two-byte-decode-encode/`
  and
  `examples/specification/run/binary-schema-packed-visible-two-byte-encode-out-of-range/`
  for two bytes.
  Closed `Dispatch(tag_field, tag => Payload, ...)` fields are eligible when
  `tag_field` names an earlier visible exact-width unsigned field and every
  case payload is an implemented exact-width unsigned primitive payload or an
  eligible nested binary schema payload named as an earlier same-module binary
  schema or a public imported binary schema through a written `use` path. The
  record contains the visible tag
  field and one payload field; for nested payload schemas the payload field
  uses the nested schema decoded record shape. The helper chooses the case
  from the encoded tag value, writes selected nested payload schemas through
  the generated schema helper path in declaration order, and returns
  `Err(EncodeError("codec.dispatch_unknown_tag",
  field_path, reason))` when the tag value has no case.
  Extension-tolerant
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields are
  eligible for the same exact-width unsigned primitive or eligible nested
  binary schema payload cases when both the tag and length fields are earlier
  visible exact-width unsigned fields. The payload record field is
  `SchemaDispatchPayload<T>`, where `T`
  is the selected primitive `Int` or nested schema decoded record shape.
  `Known(value)` writes the payload selected by the visible tag field.
  `Unknown(tag, payload)` writes the bounded raw bytes from the `ByteView`
  only when the visible tag value is not a known case and matches the unknown
  payload tag.
  Same-module recursive known payload cases and public imported recursive
  payload schemas named through a written `use` path use the same
  selected-mapping eligibility as recursive closed dispatch; the generated
  encode helper projects the selected known value to the recursive payload,
  writes it through the same schema helper path, and validates the resulting
  byte count against the explicit length field.
  The supplied length field remains explicit: the helper rejects values whose
  encoded payload byte count differs from the earlier length field with
  `Err(EncodeError("codec.dispatch_length_mismatch", field_path, reason))`.
  Visible tag and payload variant disagreements return
  `Err(EncodeError("codec.dispatch_mismatch", field_path, reason))`.
  The helper writes fields in declaration order into one immutable
  `ByteChunk`, using each primitive's declared byte order, and returns
  `Result<ByteChunk, EncodeError>`. `UInt16le`, `UInt24le`, `UInt31le`,
  `UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le` emit
  little-endian bytes and use the same representability boundaries as their
  matching unsigned widths. `UInt40be` emits big-endian five-byte values,
  `UInt48be` emits big-endian six-byte values, `UInt56be` emits big-endian
  seven-byte values, and `UInt64be` emits big-endian eight-byte values.
  Standalone visible `UInt1` through `UInt7` fields emit one byte with the
  value in the declared low bits. Values outside the primitive range return
  `Err(EncodeError("codec.encode_value_unrepresentable", field_path,
  reason))`; nested schema encode failures keep the nested schema field path.
  `UInt31be` and `UInt31le` use the 31-bit maximum even though they occupy four
  bytes.
  After the helper has projected the input value to schema-local visible
  fields and checked primitive, fixed-field, length, repeat, and dispatch
  representability, it evaluates supported field-local `where` predicates in
  declaration order over the current visible `Int` field and earlier visible
  `Int` fields. Primitive representation failures and other encode
  representability failures win before field-local predicate failures. A
  failed encode predicate returns
  `Err(EncodeError("schema.validation_failed", field_path, reason))` and
  command value diagnostics preserve the schema field path, predicate text,
  owning field value, and available schema-local `Int` values.
  When a `veln run` entry returns these generated `EncodeError` values
  directly, including as direct `Result<_, EncodeError>` failures, command
  diagnostics preserve the source-visible
  `EncodeError(id, field_path, reason)` shape and attach
  `details.value_diagnostic` for
  `codec.encode_value_unrepresentable`, `codec.dispatch_unknown_tag`,
  `codec.dispatch_length_mismatch`, `codec.dispatch_mismatch`, and
  encode-time `schema.validation_failed`. Human
  output keeps the primary message focused on the failed encode fact and
  reports field path, predicate or reason details, and rendered result value
  as related notes.
  A generated encode failure can also be projected at a source-visible
  reporting boundary as
  `Err(RuntimeDiagnostic(id, message, RuntimeValueDiagnostic(field_path,
  reason)))`. In that form the command keeps the rendered
  `RuntimeDiagnostic(...)` as the result value and attaches the same
  `details.value_diagnostic` fields for supported generated encode ids such
  as `codec.encode_value_unrepresentable`.
  When a `veln run` entry returns
  `EncodeStep::Invalid(EncodeError(id, field_path, reason))`, the command
  reports the contained `EncodeError` through the same command-facing value
  diagnostic projection. `Encoded` and `Partial` remain ordinary successful
  source-visible values.
  Unsupported non-byte-aligned reserved-bit encode shapes report
  `schema.reserved_bits_encode` at the reserved field span before typed IR is
  emitted. The diagnostic message names the rejected
  `ReservedBits(width, value)` layout; details name the schema, field,
  reserved width, expected value, supported layout family, and adjacent visible
  field widths when an exact-width visible field is adjacent. Human output
  keeps the same facts in related notes.
  Multiple selected mapping clauses selected by `when field == literal` or
  `when field != literal` are eligible when all clauses resolve to the same
  target record shape and every schema-local encode field, including the
  selector field, projects back from the selected target record through direct
  source-field assignments. The helper selects the mapping whose projected
  selector value satisfies the clause, then uses the same generated encode
  diagnostic shape for selector and projected-field representation failures.
  Closed dispatch fields whose cases mix primitive and nested schema payload
  decoded shapes are eligible at the same selected mapping boundary when all
  selectors use the dispatch tag field, cover each closed case exactly once,
  and resolve to one target record shape. The generated encode helper
  projects the selected target value back to the schema-local tag and payload
  fields for that case, then writes primitive payloads through the selected
  primitive encode path and nested payloads through the nested schema helper
  path. Unknown tag values report `codec.dispatch_unknown_tag`; known tags
  paired with the wrong selected payload mapping report
  `codec.dispatch_mismatch`.
  Same-module recursive
  closed-dispatch and extension-dispatch payload cases and public imported
  recursive payload schemas named through a written `use` path are also
  eligible in the length-bounded form when selected mappings cover every
  dispatch case, all mappings resolve to one record shape, and at least one
  case is non-recursive. The generated encode helper writes the selected
  recursive payload through the same schema helper path and checks the encoded
  payload byte count against the earlier length field. This slice excludes
  selected mappings that cannot reconstruct all schema-local encode fields,
  mapping expressions that cannot be projected back to schema-local fields,
  recursive dispatch payload schemas outside that selected same-module or
  public imported length-bounded dispatch slice, dispatch payload schemas
  outside the generated helper slice, nested mappings, and derived codec
  encode execution for unsupported schemas. Ineligible resolved nested payload
  schemas are rejected before helper generation; the checked
  `schema.dispatch_payload` diagnostics include payload schemas whose mapping
  assignments decode but cannot project back to schema-local encode fields.
  The checked examples are
  `examples/specification/run/binary-schema-u64-widths-encode/`,
  `examples/specification/run/binary-schema-u64-widths-encode-out-of-range/`,
  `examples/specification/run/binary-schema-sub-byte-encode/`,
  `examples/specification/run/binary-schema-sub-byte-encode-human/`,
  `examples/specification/run/binary-schema-sub-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-sub-byte-encode-out-of-range-human/`,
  `examples/specification/run/binary-schema-packed-visible-byte-decode-encode/`,
  `examples/specification/run/binary-schema-packed-visible-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-primitive-encode/`,
  `examples/specification/run/binary-schema-flag8-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag16be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag16le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag24-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag32be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag32le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag40be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag40le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag48be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag48le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag56be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag56le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag64be-mapped-record-encode/`,
  `examples/specification/run/binary-schema-flag64le-mapped-record-encode/`,
  `examples/specification/run/binary-schema-mapped-record-encode/`,
  `examples/specification/run/binary-schema-primitive-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag8-encode/`,
  `examples/specification/run/binary-schema-flag16be-encode/`,
  `examples/specification/run/binary-schema-flag16le-encode/`,
  `examples/specification/run/binary-schema-flag24-encode/`,
  `examples/specification/run/binary-schema-flag32be-encode/`,
  `examples/specification/run/binary-schema-flag32le-encode/`,
  `examples/specification/run/binary-schema-flag40be-encode/`,
  `examples/specification/run/binary-schema-flag40le-encode/`,
  `examples/specification/run/binary-schema-flag48be-encode/`,
  `examples/specification/run/binary-schema-flag48le-encode/`,
  `examples/specification/run/binary-schema-flag56be-encode/`,
  `examples/specification/run/binary-schema-flag56le-encode/`,
  `examples/specification/run/binary-schema-flag64be-encode/`,
  `examples/specification/run/binary-schema-flag64le-encode/`,
  `examples/specification/run/binary-schema-flag8-bit-helpers/`,
  `examples/specification/run/binary-schema-flag8-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag16be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag16le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag24-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag32be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag32le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag40be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag40le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag48be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag48le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag56be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag56le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag64be-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag64le-encode-out-of-range/`,
  `examples/specification/run/binary-schema-flag8-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-flag8-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-int-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-int-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-multi-payload-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-multi-payload-mapped-constructor-encode-mismatch/`,
  `examples/specification/run/binary-schema-mapped-constructor-field-selection-encode/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-mismatch/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-mismatch-json/`,
  `examples/specification/run/binary-schema-record-payload-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode-outer-mismatch-json/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode-inner-mismatch-json/`,
  `examples/specification/run/binary-schema-nested-mapped-constructor-encode-out-of-range/`,
  `examples/specification/run/binary-schema-byteview-encode/`,
  `examples/specification/run/binary-schema-byteview-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-byteview-add-decode/`,
  `examples/specification/run/binary-schema-byteview-add-truncated-json/`,
  `examples/specification/run/binary-schema-byteview-add-encode/`,
  `examples/specification/run/binary-schema-byteview-add-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-byteview-product-decode/`,
  `examples/specification/run/binary-schema-byteview-product-truncated-json/`,
  `examples/specification/run/binary-schema-byteview-product-encode/`,
  `examples/specification/run/binary-schema-byteview-product-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-byteview-quotient-encode/`,
  `examples/specification/run/binary-schema-byteview-quotient-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-byteview-subtract-decode/`,
  `examples/specification/run/binary-schema-byteview-subtract-negative-json/`,
  `examples/specification/run/binary-schema-byteview-subtract-truncated-json/`,
  `examples/specification/run/binary-schema-byteview-subtract-encode/`,
  `examples/specification/run/binary-schema-byteview-subtract-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-repeat-encode/`,
  `examples/specification/run/binary-schema-repeat-add-encode/`,
  `examples/specification/run/binary-schema-repeat-subtract-encode/`,
  `examples/specification/run/binary-schema-repeat-product-encode/`,
  `examples/specification/run/binary-schema-repeat-quotient-encode/`,
  `examples/specification/run/binary-schema-repeat-encode-out-of-range/`,
  `examples/specification/run/binary-schema-repeat-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-add-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-subtract-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-product-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-quotient-encode-count-mismatch/`,
  `examples/specification/run/binary-schema-repeat-nested-encode/`,
  `examples/specification/run/binary-schema-imported-repeat-nested-encode/`,
  `examples/specification/run/binary-schema-repeat-nested-encode-failure/`,
  `examples/specification/run/binary-schema-repeat-byteview-encode/`,
  `examples/specification/run/binary-schema-repeat-byteview-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-reserved-bit-encode/`,
  `examples/specification/run/binary-schema-reserved-byte-prefix-decode-encode/`,
  `examples/specification/run/binary-schema-reserved-nine-bit-prefix-decode-encode/`,
  `examples/specification/run/binary-schema-reserved-nine-bit-prefix-json/`,
  `examples/specification/run/binary-schema-reserved-nine-bit-prefix-truncated-json/`,
  `examples/specification/run/binary-schema-packed-reserved-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-four-byte-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-four-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-packed-reserved-three-byte-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-suffix-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-suffix-encode-out-of-range/`,
  `examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-encode/`,
  `examples/specification/run/binary-schema-packed-reserved-two-byte-suffix-encode-out-of-range/`,
  `examples/specification/run/binary-schema-five-byte-reserved-suffix-decode-encode/`,
  `examples/specification/run/binary-schema-five-byte-reserved-suffix-json/`,
  `examples/specification/run/binary-schema-six-byte-reserved-suffix-decode-encode/`,
  `examples/specification/run/binary-schema-six-byte-reserved-suffix-json/`,
  `examples/specification/run/binary-schema-six-byte-reserved-suffix-truncated-json/`,
  `examples/specification/run/binary-schema-six-byte-reserved-suffix-encode-out-of-range/`,
  `examples/specification/run/binary-schema-packed-reserved-two-byte-encode-out-of-range/`,
  `examples/specification/run/binary-schema-middle-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-byte-interleaved-middle-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-byte-interleaved-middle-reserved-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-byte-group-encode-out-of-range/`,
  `examples/specification/run/binary-schema-suffix-reserved-group-decode-encode/`,
  `examples/specification/run/binary-schema-suffix-reserved-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-three-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-three-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-four-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-four-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-four-byte-group-truncated-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-four-byte-group-high-encode-out-of-range/`,
  `examples/specification/run/binary-schema-prefix-reserved-four-byte-group-low-encode-out-of-range/`,
  `examples/specification/run/binary-schema-prefix-reserved-five-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-five-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-five-byte-group-human/`,
  `examples/specification/run/binary-schema-prefix-reserved-five-byte-group-encode-out-of-range/`,
  `examples/specification/run/binary-schema-prefix-reserved-six-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-six-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-six-byte-group-human/`,
  `examples/specification/run/binary-schema-prefix-reserved-six-byte-group-encode-out-of-range/`,
  `examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-human/`,
  `examples/specification/run/binary-schema-prefix-reserved-seven-byte-group-encode-out-of-range/`,
  `examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-decode-encode/`,
  `examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-json/`,
  `examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-human/`,
  `examples/specification/run/binary-schema-prefix-reserved-eight-byte-group-encode-out-of-range/`,
  `examples/specification/run/binary-schema-split-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-interleaved-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-interleaved-reserved-json/`,
  `examples/specification/run/binary-schema-five-byte-split-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-five-byte-split-reserved-json/`,
  `examples/specification/run/binary-schema-five-byte-split-reserved-human/`,
  `examples/specification/run/binary-schema-six-byte-split-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-six-byte-split-reserved-json/`,
  `examples/specification/run/binary-schema-six-byte-split-reserved-human/`,
  `examples/specification/run/binary-schema-seven-byte-split-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-seven-byte-split-reserved-json/`,
  `examples/specification/run/binary-schema-seven-byte-split-reserved-human/`,
  `examples/specification/run/binary-schema-eight-byte-split-reserved-decode-encode/`,
  `examples/specification/run/binary-schema-eight-byte-split-reserved-json/`,
  `examples/specification/run/binary-schema-eight-byte-split-reserved-human/`,
  `examples/specification/run/binary-schema-middle-reserved-json/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-recursive-closed-dispatch-encode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`,
  `examples/specification/run/binary-schema-dispatch-byteview-payload-encode/`,
  `examples/specification/run/binary-schema-dispatch-reserved-payload-roundtrip/`,
  `examples/specification/run/binary-schema-imported-closed-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-imported-dispatch-byteview-payload-encode/`,
  `examples/specification/run/binary-schema-imported-recursive-dispatch-encode/`,
  `examples/specification/run/binary-schema-mixed-dispatch-selected-mapping-encode/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode-unknown-tag/`,
  `examples/specification/run/binary-schema-closed-dispatch-encode-out-of-range/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-dispatch-nested-general-helper-encode/`,
  `examples/specification/run/binary-schema-dispatch-byteview-payload-encode/`,
  `examples/specification/run/binary-schema-dispatch-reserved-payload-roundtrip/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode/`,
  `examples/specification/run/binary-schema-imported-dispatch-byteview-payload-encode/`,
  `examples/specification/run/binary-schema-imported-extension-dispatch-nested-encode-unknown/`,
  `examples/specification/run/binary-schema-recursive-extension-dispatch-encode/`,
  `examples/specification/run/binary-schema-imported-recursive-dispatch-encode/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-mismatch/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-tag-mismatch/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-out-of-range/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode-length-mismatch/`,
  `examples/specification/run/binary-schema-dispatch-nested-encode-failure/`,
  `examples/specification/run/binary-schema-imported-dispatch-nested-encode-failure/`,
  `examples/specification/run/binary-schema-encode-value-diagnostic-json/`,
  `examples/specification/run/binary-schema-encode-value-diagnostic-human/`,
  `examples/specification/run/binary-schema-encode-validation-json/`,
  `examples/specification/run/binary-schema-mapped-encode-validation-human/`,
  `examples/specification/run/binary-schema-dispatch-unknown-tag-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-unknown-tag-encode-diagnostic-human/`,
  `examples/specification/run/binary-schema-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-length-encode-diagnostic-human/`,
  `examples/specification/run/binary-schema-recursive-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-recursive-extension-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-imported-recursive-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-imported-recursive-extension-dispatch-length-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-mismatch-encode-diagnostic-json/`,
  `examples/specification/run/binary-schema-dispatch-mismatch-encode-diagnostic-human/`,
  `examples/specification/run/binary-schema-general-helper-roundtrip/`,
  and
  `examples/specification/check/schema-reserved-bit-encode-diagnostics/`.
- A codec declaration with a valid `derive encode` clause for the same
  eligible generated binary schema encode helper slice exposes the codec item
  name as the executable encode boundary for ordinary source calls, including
  opt-in visible flag bitset fields, visible-only packed two-byte groups,
  repeat-backed schemas,
  quotient-count repeat schemas, additive, subtractive, product-sized, and
  quotient-sized `ByteView` payload fields, the implemented
  direct structural mapping and selected structural mapping slices, eligible
  nested dispatch payload schemas, and same-module recursive closed and
  extension dispatch payload helpers already accepted by
  `byte_encode_<schema>`.
  The call accepts the generated helper's value record or mapped target
  record, invokes the schema encode helper, returns `EncodeStep<()>`, projects
  helper `Ok(ByteChunk)` output to `Encoded(List<ByteChunk>)` with one chunk,
  and projects helper `Err(EncodeError)` output to `Invalid(EncodeError)`.
  The checked examples are
  `examples/specification/run/derived-codec-encode-boundary/`,
  `examples/specification/run/derived-codec-budgeted-encode-boundary/`,
  `examples/specification/run/derived-codec-mapped-encode-boundary/`,
  `examples/specification/run/derived-codec-mapping-arithmetic-encode-boundary/`,
  `examples/specification/run/derived-codec-mapped-converter-encode-boundary/`,
  `examples/specification/run/derived-codec-selected-mapping-encode-boundary/`,
  `examples/specification/run/derived-codec-mixed-dispatch-selected-mapping-encode-boundary/`,
  `examples/specification/run/derived-codec-record-payload-mapped-encode-boundary/`,
  `examples/specification/run/derived-codec-flag-boundary/`,
  `examples/specification/run/derived-codec-byteview-encode-boundary/`,
  `examples/specification/run/derived-codec-byteview-add-subtract-boundary/`,
  `examples/specification/run/derived-codec-byteview-product-boundary/`,
  `examples/specification/run/derived-codec-repeat-encode-boundary/`,
  `examples/specification/run/derived-codec-repeat-byteview-encode-boundary/`,
  `examples/specification/run/derived-codec-repeat-quotient-boundary/`,
  `examples/specification/run/derived-codec-packed-visible-two-byte-boundary/`,
  `examples/specification/run/derived-codec-nested-dispatch-encode-boundary/`,
  `examples/specification/run/derived-codec-imported-nested-dispatch-encode-boundary/`,
  `examples/specification/run/derived-codec-recursive-dispatch-boundary/`,
  `examples/specification/run/derived-codec-general-helper-boundary/`,
  `examples/specification/run/derived-codec-split-reserved-boundary/`,
  `examples/specification/run/derived-codec-six-byte-reserved-suffix-boundary/`,
  `examples/specification/run/derived-codec-wide-reserved-prefix-boundary/`,
  and
  `examples/specification/run/binary-schema-general-helper-roundtrip/`.
  The recursive dispatch boundary case covers same-module recursive closed and
  extension dispatch payload helpers through `derive encode`.
  The general-helper roundtrip case covers the combined non-HTTP schema shape
  and checks both successful `Ok(ByteChunk)` projection and helper
  `Err(EncodeError)` projection.
  The budgeted boundary case calls the same derived codec with the value
  record plus an explicit `ByteCount` output budget. If the generated
  `ByteChunk` fits in the budget, the call returns
  `Encoded(List<ByteChunk>)`; if the chunk exceeds the budget, the call
  returns `Partial(List<ByteChunk>, ByteCount, state)` with the emitted prefix,
  produced byte count, and a state record that carries the original value
  fields plus `encoded_offset: ByteCount`. Passing that state record back to
  the same codec with a later budget resumes at the committed offset. Helper
  `Err(EncodeError)` output still projects to `Invalid(EncodeError)` before
  any output chunk is exposed.
  A `derive encode` clause is rejected with
  `codec.derive_helper_unsupported` when the referenced schema cannot expose
  the required generated encode helper, including mapping expression shapes
  that cannot be projected back to the schema-local encode record.
  `examples/specification/check/derived-codec-mapping-boundary-diagnostics/`
  and
  `examples/specification/check/derived-codec-helper-eligibility-diagnostics/`
  pin those checker boundaries.
- A codec declaration with a valid `derive decode` clause for the same
  eligible generated binary schema decode-step slice exposes the codec item
  name as the executable decode boundary for ordinary source calls, including
  opt-in visible flag bitset fields,
  supported middle reserved-bit layouts, including byte-interleaved middle
  reserved layouts, wide reserved prefix groups, visible-only packed
  two-byte groups, repeat-backed
  schemas, quotient-count repeat schemas, quotient-sized
  `ByteView(left_length / right_length)` payload fields, product-sized
  `ByteView(left_length * right_length)` payload fields,
  additive `ByteView(left_length + right_length)` payload fields,
  subtractive `ByteView(left_length - right_length)` payload fields,
  same-module or public imported nested dispatch payload schemas, same-module
  recursive closed and extension dispatch payload helpers, and multiple
  decoded-field selected schema mappings already accepted by
  `byte_decode_step_<schema>`.
  The call accepts a bounded
  `ByteView` and explicit base `ByteOffset` and returns the same
  `DecodeStep<T>` value as
  `byte_decode_step_<schema>`, including mapped record values,
  `NeedMore(NeedBytes(count))`, and `Invalid` without consumed bytes. The
  checked examples are
  `examples/specification/run/derived-codec-decode-boundary/`,
  `examples/specification/run/derived-codec-middle-reserved-decode-boundary/`,
  `examples/specification/run/derived-codec-interleaved-reserved-decode-boundary/`,
  `examples/specification/run/derived-codec-split-reserved-boundary/`,
  `examples/specification/run/derived-codec-repeat-decode-boundary/`,
  `examples/specification/run/derived-codec-repeat-byteview-decode-boundary/`,
  `examples/specification/run/derived-codec-repeat-quotient-boundary/`,
  `examples/specification/run/derived-codec-byteview-add-subtract-boundary/`,
  `examples/specification/run/derived-codec-byteview-quotient-decode-boundary/`,
  `examples/specification/run/derived-codec-byteview-product-boundary/`,
  `examples/specification/run/derived-codec-flag-boundary/`,
  `examples/specification/run/derived-codec-packed-visible-two-byte-boundary/`,
  `examples/specification/run/derived-codec-nested-dispatch-decode-boundary/`,
  `examples/specification/run/derived-codec-imported-nested-dispatch-decode-boundary/`,
  `examples/specification/run/derived-codec-recursive-dispatch-boundary/`,
  `examples/specification/run/derived-codec-general-helper-boundary/`,
  `examples/specification/run/derived-codec-six-byte-reserved-suffix-boundary/`,
  `examples/specification/run/derived-codec-wide-reserved-prefix-boundary/`,
  and
  `examples/specification/run/binary-schema-general-helper-roundtrip/`.
  The recursive dispatch boundary case covers same-module recursive closed and
  extension dispatch payload helpers through `derive decode`, including
  successful recursive decode, short-input `NeedMore`, helper failure
  `Invalid`, and extension unknown-payload preservation.
  The general-helper roundtrip case covers the combined non-HTTP schema shape
  and checks successful `Decoded`, short-input `NeedMore`, and helper-failure
  `Invalid` outcomes through the codec item.
  `examples/specification/run/codec-needmore-parser-state/` covers
  caller-owned parser state around the codec boundary: after `Decoded`, the
  caller drops exactly the consumed prefix and advances the explicit base
  offset by the consumed count; after `NeedMore`, the caller keeps the same
  pending bytes and base offset.
  `examples/specification/run/codec-selected-mapping-decode-boundary/`
  covers the selected mapping boundary shared with hand-written decode
  codecs. For the implemented structural mapping slice, `T` is the mapping
  target record shape when each assignment source has the same implemented
  decoded field type as the target field and all selected mappings resolve to
  that same record shape.
  A `derive decode` clause is rejected with
  `codec.derive_helper_unsupported` when the referenced schema cannot expose
  the required generated decode-step helper. The helper eligibility diagnostics
  case listed above pins that checker boundary.
- A codec declaration with a valid hand-written `decode with function_name`
  clause exposes the codec item name as the executable decode boundary for
  ordinary source calls. The call accepts a bounded `ByteView` and explicit
  base `ByteOffset` and invokes the referenced same-module function.
  `NeedMore(readiness)` and `Invalid(error)` return unchanged.
  `Decoded(value, consumed)` returns unchanged when `consumed` is within the
  supplied view length; when `consumed` is outside the supplied view, the codec
  boundary returns `Invalid(DecodeError("codec.consumed_count_invalid",
  base_offset, codec_name))`. When the referenced schema uses multiple
  decoded-field selected mappings that resolve to one implemented target
  record shape, the referenced function must return `DecodeStep<T>` for that
  selected mapping record shape. The checked
  `examples/specification/run/codec-byteview-offset-needmore/` case covers a
  source-written decoder that inspects a bounded `ByteView`, returns
  `Decoded` with consumed `ByteCount`, returns non-consuming
  `NeedMore(NeedBytes(...))` for short input, and reports malformed input with
  a `DecodeError` whose byte offset is the caller-supplied base offset plus
  the local field position.
- For `veln run` entries, a returned
  `Err(RuntimeDiagnostic(id, message, RuntimeByteDiagnostic(...)))` is a
  source-visible diagnostic-bearing result failure. The command boundary
  keeps the rendered `RuntimeDiagnostic(...)` as the result value and projects
  the contained byte diagnostic into human runtime diagnostics and
  `details.byte_diagnostic` JSON. The implemented byte detail slice carries
  `ByteOffset`, field-path segments, count/readiness, range, or reason facts,
  and an optional bounded byte preview. Plain `Err(value)` remains an ordinary
  result failure and does not opt into diagnostic projection.
- For `veln run` entries, a returned
  `Err(RuntimeDiagnostic(id, message, RuntimeHpackFixtureDiagnostic(...)))`
  is the source-visible diagnostic-bearing result failure form used by common
  HPACK fixture projections. Dynamic-index and table-size update placement
  projections use dedicated HPACK fixture detail constructors for their extra
  public facts. The command boundary keeps the rendered
  `RuntimeDiagnostic(...)` as the result value and projects byte offset,
  observed header block size, observed first byte, expected fixture, codec
  module, and bounded header-block preview into the same human diagnostic and
  `details.protocol_diagnostic` JSON shape used by the compatibility helper.
  The standard `hpack_fixture_*` reporting helpers return those HPACK fixture
  payloads directly as `Result<(), RuntimeDiagnostic>`, so direct helper
  command diagnostics are rendered from the returned value.
- For `veln run` entries, a returned HTTP/2 protocol
  `Err(RuntimeDiagnostic(...))` payload can project directly to the same
  human runtime diagnostic and `details.protocol_diagnostic` JSON shape as
  the compatibility helper. The implemented HTTP/2 slice covers
  `RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...)`,
  `RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...)`,
  `RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...)`,
  `RuntimeHttp2PeerLimitFrameSizeDiagnostic(...)`,
  `RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(...)`,
  `RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(...)`,
  `RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(...)`,
  `RuntimeHttp2PeerLimitSettingsValueDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...)`,
  `RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...)`,
  `RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...)`,
  `RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(...)`,
  `RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...)`,
  `RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...)`, and
  `RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...)`, keeping the
  rendered `RuntimeDiagnostic(...)` as the result value while projecting the
  stable id, byte offset, protocol facts, provenance, and bounded byte preview
  where the diagnostic owns one. The `http2_protocol_closed_with_pending`,
  `http2_protocol_partial_preface`, `http2_protocol_invalid_preface`,
  `http2_protocol_continuation_expected`,
  `http2_protocol_invalid_frame_kind`,
  `http2_protocol_invalid_stream_id`,
  `http2_protocol_invalid_payload_length`,
  `http2_protocol_invalid_window_update_increment`,
  `http2_protocol_invalid_data_padding`,
  `http2_protocol_content_length_mismatch`,
  `http2_protocol_unexpected_settings_ack`,
  `http2_protocol_invalid_priority_dependency`,
  `http2_protocol_stream_after_goaway`,
  `http2_peer_limit_frame_size_exceeded`,
  `http2_peer_limit_header_list_size_exceeded`,
  `http2_peer_limit_header_table_size_exceeded`,
  `http2_peer_limit_flow_control_window_exceeded`,
  `http2_peer_limit_concurrent_streams_exceeded`, and
  `http2_peer_limit_settings_value_out_of_range` standard helpers return
  their HTTP/2 protocol payloads directly as `Result<(), RuntimeDiagnostic>`.
  The checked fixed-payload-length protocol examples include SETTINGS ACK,
  PING, GOAWAY, `RST_STREAM`, and `WINDOW_UPDATE` as source-visible
  `RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...)` payloads.
- For `veln run` entries, a returned source-visible
  `DecodeError(id, byte_offset, field_path)`,
  `DecodeErrorWithReason(id, byte_offset, field_path, reason)`,
  `DecodeStep::Invalid(DecodeError(id, byte_offset, field_path))`, or
  `DecodeStep::Invalid(DecodeErrorWithReason(id, byte_offset, field_path, reason))`
  is projected to a focused human runtime diagnostic and
  `details.byte_diagnostic` JSON using the contained diagnostic id, byte
  offset, field path, optional reason, and optional byte-helper context
  carried by the reason. The carried context includes local byte offset,
  expected and available byte counts, and bounded byte preview when the helper
  produced those facts. This includes codec-owned invalid-input facts returned
  by a hand-written `decode with` codec boundary and the
  `codec.consumed_count_invalid` result produced by the hand-written codec
  boundary when a decoded consumed count is outside the supplied `ByteView`.
  A returned
  `DecodeStep::NeedMore(readiness)` is projected at the closed-input
  reporting boundary as
  `codec.incomplete_input`, with readiness and requested byte count details
  from the source-visible `DecodeReadiness` value. `Decoded` remains an
  ordinary successful entry value. The checked examples are
  `examples/specification/run/codec-decode-decoded-json/`,
  `examples/specification/run/codec-decode-consumed-count-invalid-human/`,
  `examples/specification/run/codec-decode-consumed-count-invalid-json/`,
  `examples/specification/run/codec-decode-invalid-byte-context-human/`,
  `examples/specification/run/codec-decode-invalid-byte-context-json/`,
  `examples/specification/run/codec-decode-invalid-byte-read-context-human/`,
  `examples/specification/run/codec-decode-invalid-byte-read-context-json/`,
  `examples/specification/run/codec-decode-error-direct-json/`,
  `examples/specification/run/codec-decode-error-reason-direct-json/`,
  `examples/specification/run/codec-decode-invalid-boundary-human/`,
  `examples/specification/run/codec-decode-invalid-boundary-json/`,
  `examples/specification/run/codec-decode-invalid-owned-id-human/`,
  `examples/specification/run/codec-decode-invalid-owned-id-json/`,
  `examples/specification/run/codec-decode-invalid-reason-step-human/`,
  `examples/specification/run/codec-decode-invalid-reason-step-json/`,
  `examples/specification/run/codec-decode-invalid-step-human/`,
  `examples/specification/run/codec-decode-invalid-step-json/`,
  `examples/specification/run/codec-decode-need-end-human/`,
  `examples/specification/run/codec-decode-need-end-json/`,
  `examples/specification/run/codec-decode-need-more-human/`, and
  `examples/specification/run/codec-decode-need-more-json/`.
- A codec declaration with a valid hand-written `encode with function_name`
  clause exposes the codec item name as the executable encode boundary for
  ordinary source calls. The call invokes the referenced same-module function
  with that function's parameters and returns its `EncodeStep<TState>` value
  unchanged, including `Encoded`, `Partial`, and `Invalid` results. A checked
  budgeted encode example observes `Partial` with its emitted chunk list,
  produced byte count, and resumed encoder state as ordinary source-visible
  values, then uses the returned state to complete a later encode call. For
  `veln run` entries, a returned `Invalid(EncodeError(...))` is projected to
  the same focused human and `details.value_diagnostic` JSON diagnostics used
  for command-facing `EncodeError` result values. The checked examples are
  `examples/specification/run/codec-encode-error-direct-json/`,
  `examples/specification/run/codec-encode-invalid-step-human/`, and
  `examples/specification/run/codec-encode-invalid-step-json/`. For the
  implemented structural `map to Target` schema slice, the first encoder
  parameter remains the mapped target record shape.
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
  one. A fixture may also write `schema = "Name"` or
  `schema = "module::Name"` to validate that the fixture metadata names a
  schema from the command source set. Bare names resolve in the fixture's
  source module. Qualified names require a written `use` path and a public
  schema or public schema alias. When `field_path` is present, its first
  segment must name the resolved schema. Invalid fixture schema references are
  manifest validation errors for executable specification cases, not runtime
  `veln run` output. The accepted fixture metadata reference cases cover
  same-module schemas, imported public schemas, and imported public schema
  aliases.
- Executable specification cases may also assert named output `ByteChunk`
  lists through complete lowercase hex in `case.toml`. The harness checks
  stable consecutive program-output lines for the list count, chunk order,
  exact hex strings, decoded byte counts, empty lists, and zero-length chunks.
- Executable specification cases that intentionally reject fixture schema
  metadata may write `[manifest_error]` with `contains = [...]`; the harness
  validates those substrings against the manifest validation failure and does
  not run the command.
- The first ordinary-source HTTP/2 sans-I/O protocol-core example models
  chunk arrival and end-of-stream events as ADTs. Its pure decode state keeps
  undecoded suffix bytes, the next absolute byte offset, client connection
  preface state, continuation state with accumulated opaque header-block
  bytes, the active local receive-limit entry, peer-advertised SETTINGS state,
  outstanding local SETTINGS state, and graceful shutdown state. It validates the
  client connection preface before frame-header decode and represents partial
  or mismatched prefaces, closed-input truncation, continuation ordering
  failures for different frame kinds and stream ids with the inspected frame
  header available for diagnostics, closed input while a header block remains
  pending with retained pending bytes available for diagnostics, completed
  HEADERS and multi-frame
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
  decoding, zero-length SETTINGS ACK frames that clear outstanding local
  SETTINGS state,
  zero-length SETTINGS ACK frames with no outstanding local SETTINGS state
  and bounded inspected frame-header previews,
  wrong-length SETTINGS ACK payloads with source-visible runtime diagnostic
  payloads and bounded inspected-payload previews,
  stream id domain failures including HEADERS and CONTINUATION on the
  connection stream with bounded inspected frame-header previews, invalid
  stream-state frame kinds with bounded inspected frame-header previews,
  wrong-length PING, GOAWAY, and `RST_STREAM` payloads with source-visible
  runtime diagnostic payloads, bounded inspected-payload previews, and
  command-facing human and JSON projection cases, wrong-length PRIORITY
  payloads with bounded inspected-payload previews, PRIORITY-flagged HEADERS
  payloads shorter than five bytes before HPACK fixture decode, accepted PING
  ACK distinction, accepted single-frame and continued HEADERS with the
  PRIORITY flag after stripping the leading priority section from the HPACK
  header-block bytes, accepted HEADERS `END_STREAM` lifecycle with the
  PRIORITY flag, HEADERS priority self-dependency failures with the inspected
  priority payload preview,
  accepted PRIORITY dependency stream id, exclusive flag, and weight facts
  recorded on the tracked open stream, replacement of those tracked priority
  facts by a later accepted PRIORITY frame for the same stream, accepted
  PRIORITY dependency stream id, exclusive flag, and weight facts on an idle
  client-initiated stream without opening a peer-created stream or changing
  the concurrent-stream receive count, including when another peer-created
  stream is already tracked as open and stays unchanged, PRIORITY
  stream-state failures for closed-by-peer and reset streams, PRIORITY
  self-dependency failures including the idle-stream case, peer-sent `PUSH_PROMISE`
  rejection,
  server-side outbound `PUSH_PROMISE` send-intents for open
  client-created streams, client-side peer-sent `PUSH_PROMISE` receive on an
  open client-created associated stream with reserved-by-peer promised stream
  state,
  accepted GOAWAY last-stream-id and error-code, GOAWAY last-stream-id
  enforcement for later peer-created HEADERS streams and local outbound
  HEADERS send-intents above a received boundary, and accepted
  `RST_STREAM` error-code facts as typed protocol values. In the server-side
  fixture core, SETTINGS,
  PING, and GOAWAY require stream id zero; HEADERS, DATA, PRIORITY, `RST_STREAM`,
  `PUSH_PROMISE`, CONTINUATION, and stream-level `WINDOW_UPDATE` require a nonzero
  client-initiated stream id. The receive flow-control state opens admitted
  idle peer-created HEADERS streams only when the active concurrent-stream
  receive limit allows the new stream. With one peer-created stream already
  open in the checked fixture boundary, a second idle HEADERS stream is
  rejected before admission through the concurrent-stream peer-limit
  diagnostic, with the endpoint role, active state, receive-limit provenance,
  and rule provenance kept as diagnostic context. It consumes DATA payload
  length from the shared connection window and the targeted stream window, accepts
  PADDED DATA by consuming the pad-length byte and padding as receive-window
  credit while exposing only application data bytes as DATA content, compares
  the total exposed DATA application byte count with the accepted
  `content-length` value when a fixture-marked request or response header
  list provided one, rejects an over-length DATA frame immediately, rejects an
  early peer `END_STREAM` shortfall, moves
  the stream to
  a closed-by-peer state when accepted inbound DATA carries `END_STREAM`, moves
  completed inbound HEADERS or CONTINUATION header blocks to the same
  closed-by-peer state when the accepted HEADERS sequence carries
  `END_STREAM`, records local `END_STREAM` send-intents as
  half-closed-local for inbound processing, accepts later inbound DATA on that
  stream with the same receive-window and PADDED DATA validation rules, moves
  that stream to closed-by-peer when the accepted inbound DATA carries peer
  `END_STREAM`, accepts
  connection-level and open-stream `WINDOW_UPDATE` increments, rejects zero
  received `WINDOW_UPDATE` increments through
  `http2.protocol.invalid_window_update_increment` with a bounded preview of
  the inspected four-byte increment payload, applies
  received `SETTINGS_INITIAL_WINDOW_SIZE` deltas to the tracked open stream's
  receive-window credit, and keeps wrong-length, idle-stream, zero,
  half-closed-local stream, closed-by-peer stream, reset-stream,
  concurrent-stream-limit,
  header-list-size, invalid DATA padding, negative-credit DATA, and overflow
  cases as typed protocol failures. Stream-window and connection-window
  inbound DATA overflow diagnostics carry a bounded preview of the inspected
  DATA payload bytes.
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
- The HTTP/2 protocol-core HPACK fixture boundary models HPACK as an imported
  ordinary source module, not as schema syntax. The fixture module accepts a
  bounded deterministic set of header-block byte fixtures, including every HPACK
  static indexed `0x81` `:authority` with an empty value, `0x82`
  `:method: GET`, `0x83` `:method: POST`, `0x84` `:path: /`, `0x85`
  `:path: /index.html`, `0x86` `:scheme: http`, and
  `0x87` `:scheme: https`, plus `0x88` `:status: 200`, `0x89`
  `:status: 204`, `0x8a` `:status: 206`, `0x8b` `:status: 304`, `0x8c`
  `:status: 400`, `0x8d` `:status: 404`, `0x8e` `:status: 500`,
  `0x8f` `accept-charset:`, `0x90` `accept-encoding: gzip, deflate`, and
  `0x91` `accept-language:`, plus `0x92` `accept-ranges:`, `0x93`
  `accept:`, `0x94` `access-control-allow-origin:`, `0x95` `age:`,
  `0x96` `allow:`, `0x97` `authorization:`, `0x98` `cache-control:`,
  plus `0x99` `content-disposition:`, `0x9a`
  `content-encoding:`, `0x9b` `content-language:`, `0x9c`
  `content-length:`, `0x9d` `content-location:`, `0x9e`
  `content-range:`, `0x9f` `content-type:`, `0xa0` `cookie:`, `0xa1`
	  `date:`, `0xa2` `etag:`, `0xa3` `expect:`, `0xa4` `expires:`,
	  `0xa5` `from:`, `0xa6` `host:`, `0xa7` `if-match:`, `0xa8`
	  `if-modified-since:`, `0xa9` `if-none-match:`, `0xaa` `if-range:`,
	  `0xab` `if-unmodified-since:`, `0xac` `last-modified:`, `0xad`
	  `link:`, `0xae` `location:`, `0xaf` `max-forwards:`, `0xb0`
	  `proxy-authenticate:`, `0xb1` `proxy-authorization:`, `0xb2`
	  `range:`, `0xb3` `referer:`, `0xb4` `refresh:`, `0xb5`
	  `retry-after:`, `0xb6` `server:`, `0xb7` `set-cookie:`, `0xb8`
	  `strict-transport-security:`, `0xb9` `transfer-encoding:`, `0xba`
	  `user-agent:`, `0xbb` `vary:`, `0xbc` `via:`, and `0xbd`
	  `www-authenticate:` bytes. The same fixture boundary accepts the
	  two-byte static-indexed block `0x82 0x84` as `:method: GET` followed
	  by `:path: /`, preserving both headers in the source-visible
	  `HpackHeaderList`. The HTTP/2 protocol-core case also carries
	  static indexed `0x85` `:path: /index.html` through a completed final
	  CONTINUATION frame before HPACK decode. The fixture boundary also
	  accepts literal-without-indexing,
	  literal-with-indexing, and literal-never-indexed fixtures whose
	  indexed-name form names a supported static-table header name already
	  accepted by the static-indexed fixture set, including ordinary names
	  such as `server`, `content-type`, and `user-agent`. Those literal
	  fixtures share the same HPACK string literal
  decoder: raw values must be visible ASCII, and Huffman-marked values
  decode by scanning the HPACK static Huffman table across the full byte
  symbol range into decoded fixture strings rather than by matching a fixed
  decoded-value allowlist. The checked Huffman string boundary accepts visible
  ASCII, the line-feed fixture value, and single-byte `hpack-byte-xx` labels
  for every byte value. Multi-byte decoded non-visible byte strings use the
  deterministic `hpack-bytes-xx-...-xx` fixture label form; the existing
  `hpack-bytes-00-ff` spelling remains the label for decoded bytes
  `0x00 0xff`, and decoded bytes `0x00 0x00` produce `hpack-bytes-00-00`.
  The fixture also accepts raw new-name literal forms when the field-name
  string itself is a raw visible-ASCII HPACK string literal; the decoded
  field name then flows into the same HTTP/2 header-list validation used for
  static-name and dynamic-name literal fixtures.
  HPACK-prefixed integers for table-size updates, dynamic-name indexes, and
  string literal lengths are decoded by the same bounded fixture foundation,
  so the checked saturated-prefix forms take the same continuation-byte path
  before their callers apply the table-size, name lookup, or string-value
  policy.
  The same decoder accepts the fixture-boundary string-length integer
  continuation form for supported literal names: checked raw and
  Huffman-marked long values use a saturated seven-bit length prefix plus one
  continuation byte through literal-without-indexing, literal-with-indexing,
  and literal-never-indexed blocks, including raw fixture values beyond the
  former checked 128-byte decode boundary. The long Huffman fixture remains a
  deterministic fixture case, not
  general HPACK Huffman streaming support.
  The same ordinary fixture module exposes
  `encode_hpack_raw_string_literal` for fixture-owned raw string literals:
  values accepted by `byte_chunk_from_visible_ascii_string` encode with the
  HPACK Huffman flag cleared, and the string length uses the HPACK integer
  prefix rules for raw string literals. The checked output keeps the existing
  short `PUT` bytes stable, accepts a visible ASCII value that was not part of
  the former fixture allowlist, and keeps the long raw `a` fixture at the
  saturated seven-bit length prefix plus one continuation byte boundary.
  Non-visible raw byte values return
  `hpack.fixture.unsupported_header_block` with expected fixture
  `fixture raw string encoding`.
  The same fixture module exposes a narrow source-visible header-list encoder
  for outbound fixture use. It accepts the supported static-indexed header
  lists, raw and checked Huffman-marked literal-without-indexing and
  literal-with-indexing lists for supported static-table names, and the
  checked request and response pseudo-header fixture lists needed by the
  outbound HTTP/2 examples. Static indexed `:method: GET` encodes to `0x82`,
  raw literal `:path: /target` encodes to `0x04 0x07 "/target"`,
  Huffman-marked literal `:path: test` encodes to
  `0x04 0x83 0x49 0x50 0x9f`, and Huffman-marked literal `:status: 200`
  encodes to `0x08 0x82 0x10 0x01`. The same encoder is table-driven for
  visible ASCII values; the checked non-allowlist `:authority: abc.test`
  literal encodes to `0x01 0x86 0x1c 0x64 0x5d 0x25 0x42 0x7f` with the HPACK
  Huffman flag set and EOS-prefix padding in the final byte. A checked
  Huffman-marked line-feed `:path` literal encodes to
  `0x04 0x84 0xff 0xff 0xff 0xf3`, a checked Huffman-marked single-NUL
  `:path` literal encodes to `0x04 0x82 0xff 0xc7`, and a checked
  Huffman-marked `hpack-byte-ff` `:path` literal encodes to
  `0x04 0x84 0xff 0xff 0xfb 0xbf`. The bounded multi-byte
  `hpack-bytes-00-ff` `:path` literal encodes to
  `0x04 0x85 0xff 0xc7 0xff 0xff 0xdd`, proving the fixture encoder can
  leave the former visible-ASCII boundary for supported fixture values. The
  checked raw-string encoder failure path keeps a multi-byte Huffman-marked
  non-visible outbound value on the fixture-owned raw string encoding failure.
  Unsupported header names return a typed HPACK fixture
  failure with expected fixture `fixture header list encoding`.
  These encode failures are fixture codec results and are not projected as
  HTTP/2 protocol diagnostics by the outbound send-intent helpers.
  The checked example covers `:authority: abc.test` through
  completed HEADERS and final CONTINUATION paths, raw `:status` through
  completed HEADERS, Huffman `:path: test`, `:path` line feed, `:path`
  single NUL, `:path` `hpack-byte-ff`, and `:path` `hpack-bytes-00-ff`
  through completed HEADERS, plus `:path` `hpack-bytes-00-ff` through a final
  CONTINUATION,
  Huffman `:status: 200` through completed HEADERS and final CONTINUATION,
  Huffman `:method: PUT` through both literal-without-indexing and
  literal-with-indexing, Huffman `:method: bad` through
  literal-without-indexing, literal-with-indexing, and literal-never-indexed,
  raw literal-never-indexed `:path` through completed HEADERS and final
  CONTINUATION, raw literal-with-indexing `:authority`, Huffman
  literal-with-indexing `:scheme: https`, and raw literal-with-indexing
  `:status`. It also checks ordinary static-name literals: raw
  literal-without-indexing `server: ok` as `0x0f 0x27 0x02 "ok"`, raw
  literal-with-indexing `content-type: text` as `0x5f 0x04 "text"` followed
  by a later `0xbe` dynamic-indexed reuse, and raw literal-never-indexed
  `user-agent: agent` through a final CONTINUATION as
  `0x1f 0x2b 0x05 "agent"`. It also checks raw new-name literal-with-indexing
  `x-trace: ok` as `0x40 0x07 "x-trace" 0x02 "ok"` followed by dynamic-indexed
  reuse, and raw new-name trailers through the HTTP/2 trailer validation
  path, including accepted lower-case `x-trace`, rejected uppercase `Server`,
  and rejected token-invalid `bad@name`. Focused human and JSON examples pin
  those raw field-name failures on the existing
  `http2.protocol.invalid_request_header_list` projection. Checked bytes also
  include literal-without-indexing `:authority: odd` as `0x01 0x03 "odd"`,
  literal-with-indexing `:method: raw` as `0x42 0x03 "raw"`,
  literal-never-indexed `:path: bot` as `0x14 0x03 "bot"`, and zero-length `:path`
  as `0x04 0x80`,
  `:path: test` as `0x04 0x83 0x49 0x50 0x9f`, `:scheme: https` as
  `0x06 0x84 0x9d 0x29 0xad 0x1f`, `:status: 200` as
  `0x08 0x82 0x10 0x01`, `:method: bad` as `0x02 0x83 0x8c 0x72 0x7f`,
  `0x42 0x83 0x8c 0x72 0x7f`, and `0x12 0x83 0x8c 0x72 0x7f`,
  `:path` `hpack-byte-ff` as `0x04 0x84 0xff 0xff 0xfb 0xbf`,
  `:path` `hpack-bytes-00-ff` as
  `0x04 0x85 0xff 0xc7 0xff 0xff 0xdd`, and
  `:authority: www.example.com` as
  `0x01 0x8c 0xf1 0xe3 0xc2 0xe5 0xf2 0x3a 0x6b 0xa0 0xab 0x90 0xf4 0xff`.
  The focused HPACK boundary also checks raw literal-never-indexed
  `:authority: abc.test` as `0x11 0x08 "abc.test"`, Huffman-marked
  literal-never-indexed `:scheme: https` as
  `0x16 0x84 0x9d 0x29 0xad 0x1f`, the same long raw and Huffman-marked
  string-length boundary through literal-never-indexed forms, and a 129-byte
  raw `:authority` value through literal-without-indexing,
  literal-with-indexing, and literal-never-indexed forms.
  The completed HEADERS path checks a valid long raw literal before the local
  header-list receive limit rejects its decoded size; the final CONTINUATION
  path checks the same boundary for a valid long Huffman-marked literal.
  Malformed string lengths including non-terminating string-length
  continuations use `hpack.fixture.malformed_string_length`. Malformed raw
  string values for supported literal names, including non-visible raw bytes
  and malformed raw `:status` literals, use
  `hpack.fixture.malformed_raw_string_value`. Malformed Huffman padding uses
  the focused `hpack.fixture.malformed_huffman_padding` id. Huffman EOS used
  as a decoded symbol uses `hpack.fixture.huffman_eos_symbol`. Multi-byte
  decoded non-visible Huffman strings are represented as `hpack-bytes-*`
  labels instead of failing solely because their decoded bytes are
  non-visible. Each
  focused HPACK fixture diagnostic records the same header-block byte offset,
  observed size, observed first byte, codec module, expected fixture, and
  bounded preview fields as other HPACK fixture diagnostics; the checked paths
  cover completed HEADERS or final CONTINUATION as appropriate. This is still
  a fixture slice, not full HPACK string or compression support. It also
  includes the narrow dynamic-table slice where `0x44 0x07 "/target"`
  inserts `:path: /target` into the returned immutable fixture state stored on
  the HTTP/2 decode state
  and a later `0xbe` indexed representation reads that entry, returns ordinary
  header-list data plus the next immutable fixture state. Later
  literal-with-indexing fixtures prepend newest-first dynamic entries while
  retaining older entries when the bounded fixture table has room; after
  `:method: PUT` and `:scheme: https` are inserted over `:path: /target`,
  `0xbe` reads the newest `:scheme: https` entry, `0xbf` reads the second
  `:method: PUT` entry, and `0xc0` reads the third retained
  `:path: /target` entry. The same bounded fixture table also supports the
  checked dynamic-name literal-with-indexing form `0x7e 0x06 "/again"` after
  `:path: /target` has been inserted: it reuses the newest dynamic entry name
  `:path`, supplies the visible-ASCII value `/again`, prepends
  `:path: /again`, and retains the older `:path: /target` entry when table
  size allows. After three retained dynamic entries exist, the fixture also
  accepts the continuation-byte indexed-name forms `0x7f 0x00 0x05 "PATCH"`
  and `0x7f 0x01 0x06 "/third"`: they reuse dynamic index values `63` and
  `64`, respectively, decode the following visible-ASCII value literal, and
  prepend the decoded header as the newest dynamic entry while the bounded
  fixture table has room. With a deeper bounded dynamic table, the same
  literal-with-indexing path accepts dynamic index value `127` through
  `0x7f 0x40 0x05 "/deep"`, proves the older retained `:path: /a` entry is
  still addressable through `0xff`, and carries the inserted `:path: /deep`
  value through both completed HEADERS and final CONTINUATION paths before
  later `0xbe` reads. The fixture reuses the same dynamic-name lookup for
  literal-without-indexing and literal-never-indexed header blocks with
  saturated four-bit indexed-name prefixes: `0x0f 0x2f 0x03 "/no"` decodes
  `:path: /no`, and `0x1f 0x2f 0x07 "/secret"` decodes
  `:path: /secret` after `:path: /target` has been inserted. After
  `:method: PUT` has also been inserted, the same non-inserting literal
  forms accept one continuation byte for dynamic index `63` and the deeper
  dynamic index value `127`: `0x0f 0x30 0x03` with value `/no` decodes
  `:path: /no`, `0x1f 0x30 0x07` with value `/secret` decodes
  `:path: /secret`, `0x0f 0x70 0x05 "/skip"` decodes `:path: /skip`, and
  `0x1f 0x70 0x07 "/secret"` decodes `:path: /secret`. Those
  forms advance the immutable fixture decode count without inserting a
  dynamic-table entry, so later `0xbe` and `0xbf` lookups from the returned
  states still read the previously inserted `:method: PUT` and
  `:path: /target` entries, while later `0xff` reads still observe
  `:path: /a` after the index `127` forms. Completed HEADERS and final
  CONTINUATION paths both carry that HPACK state before later header blocks
  are decoded, and the checked state output shows the decode count is
  unchanged while a CONTINUATION block is still pending and advances only
  after the final accepted header-block decode. A dynamic indexed `0xbe`
  lookup without prior state reports
  `hpack.fixture.dynamic_index_out_of_range` and leaves the carried HPACK
  fixture decode count unchanged; a later accepted literal-with-indexing
  block then inserts `:path: /target` and makes the following `0xbe` readable
  through the returned state. A literal-never-indexed decode without a prior
  dynamic entry still inserts no dynamic table entry, so a later `0xbe`
  lookup from that returned state reports the same dynamic-index diagnostic.
  Missing, malformed, and out-of-range dynamic-name continuations remain on
  `hpack.fixture.unsupported_header_block`. It also
  accepts dynamic table-size updates `0x3e`, `0x3f`, one-byte HPACK integer
  continuations such as `0x3f 0x01`, and the fixture-boundary slice of
  general multi-byte HPACK integer continuations with the table-size update
  prefix, such as `0x3f 0x0b`, `0x3f 0x80 0x01`, `0x3f 0x81 0x01`, and
  `0x3f 0x82 0x02`. The HPACK fixture boundary accepts those
  fixtures return next immutable fixture states with checked table sizes
  `30`, `31`, `32`, `42`, `159`, `160`, and `289`; the HTTP/2 core carries
  table-size updates at or below the active local header-table receive limit
  from either a completed HEADERS block or a final CONTINUATION block before a
  later header block is decoded, and rejects larger decoded updates, including
  a repeated current fixture table size above the local limit, through
  `http2.peer_limit.header_table_size_exceeded` with observed size, allowed
  size, frame kind, stream id, receive-limit provenance, and rule provenance.
  The diagnostic carries a bounded structured preview of the inspected
  header-block bytes separately from those facts.
  A complete dynamic table-size update after a decoded header field in the
  same completed header block is rejected through
  `hpack.fixture.table_size_update_not_at_start` on both completed HEADERS and
  final CONTINUATION paths. That diagnostic records the requested table size,
  frame kind, stream id, active HPACK fixture state, expected fixture
  boundary, codec module, and bounded header-block preview.
  This is not full HPACK compression support. When
  reducing the table size below the supported fixture entries, the bounded
  eviction policy measures each accepted dynamic entry as header name byte
  count plus value byte count plus `32` and evicts oldest entries first: a
  reduction to `86` keeps the newest two entries while evicting the third
  retained entry, a reduction to `42` keeps the newest supported
  `:method: PUT` entry while evicting the older `:path: /target` entry when
  those two entries are retained, the same table size evicts a supported
  `:authority: abc.test` entry, a reduction to `40` evicts the raw new-name
  ordinary `x-trace: ok` entry after its checked `0xbe` reuse path, and a
  reduction to `30` drops both supported `:method: PUT` and `:path: /target`
  entries so later dynamic indexed representations report
  `hpack.fixture.dynamic_index_out_of_range`. The checked fixture
  boundary also covers eviction caused by a later literal-with-indexing
  insertion: after a reduced table keeps `:scheme: https` and `:method: PUT`,
  inserting `:path: /target` keeps that new entry as the newest dynamic entry
  and evicts the older entries that no longer fit; a later `0xbe` lookup
  succeeds, while `0xbf` reports the dynamic-index diagnostic. That
  diagnostic records the requested dynamic index, the current dynamic table
  entry count, observed header-block size, observed first byte, codec module,
  and bounded header-block byte preview. Unsupported fixture input, including
  malformed non-terminating table-size updates,
  table-size updates with trailing bytes after a complete integer, malformed
  literal-without-indexing, projects through
  `hpack.fixture.unsupported_header_block` with the unsupported header-block
  byte offset, observed size, observed first byte, expected fixture, codec
  module, and bounded header-block byte preview carried by an ordinary
  `Err(RuntimeDiagnostic(..., RuntimeHpackFixtureDiagnostic(...)))` payload at
  the standalone HPACK fixture projection boundary. Malformed HPACK string
  lengths, malformed raw string values for supported literal names, malformed
  Huffman padding, Huffman EOS, and Huffman non-visible checked header values
  stay on the HPACK fixture boundary and can be carried by the same
  source-visible `RuntimeHpackFixtureDiagnostic` payload shape. Dynamic-index
  and table-size update placement fixture diagnostics use source-visible
  `RuntimeHpackFixtureDynamicIndexDiagnostic(...)` and
  `RuntimeHpackFixtureTableSizeUpdateDiagnostic(...)` payloads so their extra
  public facts are also carried by the returned `Err` value.
  That
  diagnostic path is
  distinct from `schema.*`, `http2.protocol.*`, and `http2.peer_limit.*` ids;
  the HTTP/2 core still owns the local
  `http2.peer_limit.header_table_size_exceeded` and
  `http2.peer_limit.header_list_size_exceeded` receive-limit boundaries after
  fixture decoding. Those receive-limit diagnostics carry bounded structured
  previews of the inspected header-block bytes. The header-list receive-limit
  boundary can carry those facts in a source-visible
  `RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(...)` payload. Fixture-marked
  request header lists are validated after that HPACK fixture decode on
  completed HEADERS and final CONTINUATION paths.
  Duplicate request pseudo-headers, request pseudo-headers after regular
  headers, missing `:method`, `:scheme`, or `:path`, response-only
  `:status`, uppercase ordinary header names, ordinary header names outside
  the HTTP field-name token shape, and connection-specific ordinary header
  names on an inbound request project through
  `http2.protocol.invalid_request_header_list`. Its primary message names the
  failed header-list fact; decoded header names, stream id, frame kind, active
  state, and rule provenance remain structured details or related notes.
  A source-visible
  `RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)` payload carries
  those request header-list facts in the returned diagnostic value.
  The standard `http2_protocol_invalid_request_header_list(...)` helper returns
  this payload directly as `Result<(), RuntimeDiagnostic>`, so direct helper
  command output is rendered from the returned value.
  Fixture-marked request `:scheme` values are valid only when they are `http`
  or `https`; any other value fails with `scheme_value_not_http_or_https`.
  Fixture-marked request `:path` values must be non-empty after `:path`
  presence is confirmed; an empty value fails with `path_value_empty`.
  The same boundary accepts ordinary `te: trailers` on inbound requests and
  rejects any other fixture-marked `te` value with failed fact
  `te_header_value_not_trailers`. Fixture-marked request `content-length`
  values are valid only when each value is a decimal byte string and repeated
  values are exactly identical. Empty, non-decimal, signed, whitespace-padded,
  and negative-looking values fail with `content_length_invalid`; repeated
  valid decimal values that differ fail with `content_length_mismatch`.
  A second inbound HEADERS block on an already-open request stream is treated
  as request trailers only when the HEADERS sequence carries peer
  `END_STREAM`; accepted trailers close the stream by peer without consuming
  connection or stream receive-window credit. The checked fixture accepts
  ordinary trailer fields through both completed HEADERS and final
  CONTINUATION paths, rejects a second HEADERS block without peer
  `END_STREAM` as a request-trailer state failure, rejects pseudo-headers
  with active state `request-trailers`, and rejects uppercase ordinary names,
  invalid field-name tokens, connection-specific ordinary names, and invalid
  `te` values through the same structured request header-list diagnostic
  fields. The raw HPACK uppercase and invalid-token trailer-name cases carry
  those facts in source-visible
  `RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)` payloads
  returned by the failing command path.
  Fixture-marked response header lists are validated at the same boundary.
  Missing or duplicate `:status`, request-only `:authority`, `:method`,
  `:scheme`, or `:path`, and response pseudo-headers after regular headers
  project through `http2.protocol.invalid_response_header_list`, as do
  uppercase ordinary header names and ordinary header names outside the HTTP
  field-name token shape on an inbound response. Ordinary `te: trailers` is
  accepted on inbound responses, and any other fixture-marked `te` value is
  rejected with failed fact `te_header_value_not_trailers`. The same
  `content-length` value rules and failed facts apply to fixture-marked
  response header lists. The response diagnostic uses the same structured
  detail shape as request validation while naming the response-specific
  failed header-list fact. A source-visible
  `RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...)` payload
  carries those response header-list facts in the returned diagnostic value.
  The standard `http2_protocol_invalid_response_header_list(...)` helper
  returns this payload directly as `Result<(), RuntimeDiagnostic>`, so direct
  helper command output is rendered from the returned value.
- The same example keeps outbound DATA send-intent flow control separate from
  inbound receive limits. Received `SETTINGS_MAX_FRAME_SIZE` constrains DATA
  payloads this endpoint sends, received `SETTINGS_INITIAL_WINDOW_SIZE`
  supplies the outbound stream credit for the peer-owned stream window, and
  valid DATA intents first check the full payload against available outbound
  connection and stream credit. Payloads larger than the peer-advertised
  maximum frame size are emitted in one immutable output chunk containing
  multiple DATA frames, each no larger than that maximum, with `END_STREAM`
  only on the final DATA frame when requested. PADDED DATA send-intents encode
  the PADDED flag, one pad-length byte per emitted frame, application bytes,
  and requested zero padding bytes; frame-size splitting and outbound credit
  checks count the full encoded DATA payload for each frame, including the
  pad-length byte and padding. Padding that cannot fit in the selected frame
  payload is rejected before output bytes or credit changes. Accepted DATA
  consumes outbound connection and stream credit by the full encoded DATA
  payload length after all split frames encode, including cases that exactly
  consume either the connection or stream send window. Over-window DATA
  intents, including zero-credit connection and stream cases, are rejected
  before output bytes or credit changes. Valid received `WINDOW_UPDATE` frames
  can refill the separate outbound connection or stream send credit so the
  same DATA payload is accepted after matching peer credit is restored. Local
  outbound `WINDOW_UPDATE` send-intents still add receive credit only and do
  not refill outbound DATA send credit. Accepted outbound DATA with
  `END_STREAM` records local closed-stream state for outbound send-intents
  while the receive core keeps the stream half-closed-local for inbound DATA.
  After accepted inbound DATA carries peer `END_STREAM`, the same example
  keeps local outbound DATA send-intents available for that closed-by-peer
  stream until local `END_STREAM` is sent. Those DATA send-intents still use
  peer-advertised outbound stream credit and peer maximum frame size; accepted
  local `END_STREAM` then records closed-stream state so later outbound DATA
  and stream-level outbound `WINDOW_UPDATE` use the closed stream-state
  rejection boundary.
  After receiving GOAWAY or after locally sending GOAWAY, outbound DATA for
  an open stream id greater than the recorded last-stream-id is rejected with
  `http2.protocol.stream_after_goaway` before frame-size splitting, encode
  checks, or outbound credit changes. DATA for an open stream at the recorded
  boundary remains accepted, while missing-stream, closed-stream,
  reset-stream, and mismatched-stream intents keep their narrower existing
  failures.
  Later outbound DATA, outbound HEADERS, and stream-level outbound
  `WINDOW_UPDATE` for that stream follow the existing closed stream-state
  rejection boundary.
  Outbound `WINDOW_UPDATE` send-intents accept connection-level and
  currently open stream-level receive-credit increments, emit one immutable
  frame with a four-byte increment payload, reject zero, negative,
  out-of-range, current-window overflow, stream id zero, idle-stream,
  closed-stream, reset-stream, and mismatched-stream intents before output
  bytes, and leave generated helper representation failures for the frame
  header or increment payload as a
  `codec.encode_value_unrepresentable` encode error rather than an HTTP/2
  diagnostic.
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
- The same example also covers the narrow local SETTINGS send-intent and ACK
  tracking slice. Ordinary source constructs one SETTINGS item for
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_INITIAL_WINDOW_SIZE`,
  `SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
  `SETTINGS_MAX_FRAME_SIZE`, or `SETTINGS_MAX_HEADER_LIST_SIZE`, and also
  covers a two-item local SETTINGS batch. Accepted local SETTINGS intents emit
  one frame-header-plus-payload output chunk with length `6 * item_count`,
  kind `4`, flags `0`, stream id `0`, and the selected setting identifier and
  four-byte unsigned value pairs in order. The connection records one
  outstanding local SETTINGS batch with the selected item count. Local
  `SETTINGS_MAX_FRAME_SIZE` accepts `16384..16777215`,
  `SETTINGS_INITIAL_WINDOW_SIZE` accepts `0..2147483647`, and
  `SETTINGS_ENABLE_PUSH` accepts `0..1`; values outside those ranges are
  rejected before bytes are emitted with the SETTINGS value range failure
  shape and `local_settings` provenance, including when the invalid value
  appears in a batch. The checked example leaves
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`, and
  `SETTINGS_MAX_HEADER_LIST_SIZE` as accepted non-negative local integer
  settings. A valid received SETTINGS ACK clears that outstanding state,
  including a multi-item batch.
  A valid received SETTINGS ACK when no local SETTINGS batch is outstanding
  fails as
  `http2.protocol.unexpected_settings_ack` with active state and rule
  provenance in related context, plus a bounded byte preview for the
  inspected frame header. The checked SETTINGS ACK failure reports that fact
  through a source-visible `RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...)`
  payload.
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
  HEADERS send-intent. Ordinary source accepts an already-encoded opaque
  header-block chunk for a nonzero currently open stream. It can also build
  that opaque header-block chunk from fixture-owned ordinary header-list
  values through the HPACK fixture encoder before entering the same send-intent
  path, including checked Huffman-marked `:path: test` and
  `:authority: abc.test` fixture literals. The checked stateful encoder path
  starts from a separate fixture encode state, emits a supported
  literal-with-indexing `:path: /target` header list, carries the returned
  bounded dynamic-table state, and emits a later matching header list as the
  dynamic indexed byte `0xbe` before the HEADERS frame-splitting boundary.
  The focused HPACK boundary also pins those stateful encode transitions and
  outbound table-size update transitions before HTTP/2 framing is involved.
  The same outbound fixture encoder accepts requested dynamic table-size
  updates before HEADERS framing, including one-byte prefix output `0x3e` for
  table size `30` and saturated-prefix continuation output `0x3f 0x81 0x01`
  for table size `160`. The returned encode state carries the reduced table
  capacity into a later outbound HEADERS encode: after reducing the table to
  `30`, the supported `:path: /target` fixture is encoded as a literal
  header block rather than reusing the earlier dynamic indexed byte. A
  requested outbound table-size update greater than the active
  peer-advertised `SETTINGS_HEADER_TABLE_SIZE` fails at the HPACK fixture
  encode boundary before HEADERS header-block bytes are emitted.
  When the header-block fits within the peer-advertised maximum frame
  size, the intent emits one immutable output chunk with a HEADERS frame header
  kind `1`,
  `END_HEADERS` set, and an optional `END_STREAM` flag, followed by the
  header-block bytes. When the header-block is larger, the same output chunk
  contains one HEADERS frame followed by as many CONTINUATION frames as needed;
  every payload chunk respects the peer-advertised maximum frame size,
  `END_HEADERS` is set only on the final frame, and optional `END_STREAM` is
  set only on the first HEADERS frame. Accepted `END_STREAM` records local
  closed-stream state so a later stream-level `WINDOW_UPDATE` for that stream
  follows the existing closed stream-state boundary. Stream id `0`, missing
  streams, closed streams, already reset streams, mismatched open streams, and
  generated frame-header representation failures are rejected before accepted
  output bytes are produced. After receiving GOAWAY or after locally sending
  GOAWAY, outbound HEADERS for an open stream id greater than the recorded
  last-stream-id are rejected with `http2.protocol.stream_after_goaway`
  before frame splitting or encode checks; HEADERS for an open stream at the
  boundary remain accepted, and stream id zero plus closed stream cases keep
  their narrower existing failures.
- The same HTTP/2 protocol-core example also covers the narrow server-side
  outbound `PUSH_PROMISE` send-intent. Ordinary source accepts a nonzero
  currently open client-created associated stream, a nonzero server-initiated
  promised stream id, and already-encoded opaque header-block bytes. It can
  also use the HPACK fixture encoder to build those header-block bytes from a
  fixture-owned header list before applying the same stream-id, frame-size,
  and CONTINUATION rules, including the checked Huffman-marked
  `:status: 200` fixture literal. When the four-byte promised stream id plus
  header block fits within the
  peer-advertised maximum frame size, the intent emits one immutable output
  chunk with a `PUSH_PROMISE` frame header kind `5`, `END_HEADERS` set, the
  associated stream id, the generated `UInt31be` promised-stream payload, and
  the header block bytes. Larger payloads use one `PUSH_PROMISE` frame
  followed by CONTINUATION frames on the associated stream; `END_HEADERS` is
  set only on the final frame. Associated stream id `0`, missing streams,
  closed streams, already reset streams, mismatched open streams,
  server-created associated streams, promised stream id `0`, and
  representable promised client-initiated stream ids are rejected before
  accepted output bytes are produced. Promised stream ids outside the
  generated payload helper's representable range stay as
  `codec.encode_value_unrepresentable` failures with the generated field
  path.
- The same HTTP/2 protocol-core example also covers the narrow client-side
  peer-sent `PUSH_PROMISE` receive slice. A client receive fixture state marks
  the associated client-created stream as open for this boundary. The receive
  path accepts a `PUSH_PROMISE` frame on that stream when the payload carries
  a nonzero server-initiated promised stream id followed by a supported HPACK
  fixture request header block. It strips the four-byte promised-stream
  payload before routing the header block through the same completed HEADERS
  and final CONTINUATION HPACK fixture decode paths used by ordinary header
  blocks, and records the promised stream as reserved by peer while keeping
  later DATA and HEADERS behavior for that promised stream outside this
  slice. Associated stream id zero and wrong-parity associated stream ids keep
  the existing stream-id diagnostic route. Promised stream id zero and
  representable client-initiated promised stream ids use
  `http2.protocol.invalid_stream_id` with client receive rule provenance.
  Payloads shorter than the promised-stream field use
  `http2.protocol.invalid_payload_length`, and unsupported promised header
  blocks keep the existing HPACK fixture diagnostic shape.
- The same HTTP/2 protocol-core example also covers the outbound GOAWAY
  send-intent. Ordinary source validates the selected last stream id and
  error code through a schema-declared `Http2GoawayPayloadWire` payload record
  encoded by the generated `byte_encode_<schema>` helper path, then emits one
  immutable output chunk with a nine-byte frame header length `8`, kind `7`,
  flags `0`, and stream id `0` followed by the eight-byte GOAWAY payload. An
  accepted intent
  records local graceful-shutdown state so a later peer-created HEADERS stream
  greater than the sent last stream id is rejected through the post-GOAWAY
  stream rule, and later local outbound HEADERS above the sent last stream id
  are rejected through the same rule before frame splitting or encode checks.
  Last-stream-id and error-code values outside the generated
  schema payload helper's representable ranges stay as
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
