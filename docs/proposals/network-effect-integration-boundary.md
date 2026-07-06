# Network Effect Integration Boundary

Status: proposed

This proposal tracks remaining work between a pure sans-I/O protocol core and
transport integration. Current implemented transport, channel, task, deadline,
and cancellation slices are specified by `../specification/names-effects.md`
and `../specification/execution.md`; completed proposal records live under
`../reference/implemented-proposals/`. This page keeps the larger transport
adapter, richer stream routing, and socket work open while treating deadline
and cancellation behavior as complete for this proposal at the current
owner/token/status/outcome boundary.

## Problem

HTTP/2 eventually needs sockets, deadlines, task scheduling, channels, and
adapter-owned routing, but the design driver starts with a pure core. The
language recognizes broad effect labels such as `net`, `time`, and
`concurrency`, and remaining transport work should build on those labels
before adding finer-grained network access modes.

The project needs a clear boundary so binary schema work does not accidentally
commit to a full network runtime.

## Remaining Scope

Define future integration support beyond the implemented descriptor-backed
boundary calls, first fixture-backed listener/stream calls, narrow socket and
stream routing slices, checked receiver-list channel-first routing slices,
checked task slices, and narrow deadline and cancellation slices, for:

- production socket ownership and lifecycle beyond the checked
  production-loopback listen, sequential accept, read, write, clean listener
  end, close lifecycle, two-stream adapter handler/action lifecycle,
  listener-drain adapter lifecycle, listener-drain read-failure boundary,
  adapter-owned outbound write-failure boundary,
  deadline-aware adapter lifecycle, deadline-aware accept-failure boundary,
  deadline-aware read-failure boundary, production cancellable deadline-aware
  adapter lifecycle and outcome boundary, explicit listener-close boundary,
  adapter-owned cancellation owner lifecycle boundary, and production
  owner-drain cancellable deadline lifecycle boundary, production
  two-stream multi-cycle routing boundary, production multi-chunk routing
  read-failure boundary, production multi-event adapter task-helper boundary,
  per-stream task handler-failure lifecycle boundary, accepted-stream address
  metadata boundary, listener endpoint text inspection boundary,
  source-visible client connect boundary, the
  stream state inspection boundary, the
  fixture-backed listen, optional accept, deadline-aware optional accept,
  optional stream-read, deadline-aware optional stream-read, cancellable
  deadline-aware stream-read, deadline-aware stream-write, cancellable
  deadline-aware stream-write, deadline-aware chunk-list stream-write,
  cancellable deadline-aware chunk-list stream-write, ordered write lifecycle
  slice, and checked
  adapter-owned listener-to-clean-stream-end, deadline-aware accepted-stream
  lifecycle, cancellable accepted-stream lifecycle, cancellable
  deadline-aware accepted-stream lifecycle, explicit stream close lifecycle,
  adapter-owned clean shutdown lifecycle, explicit listener close lifecycle,
  production multi-chunk event routing, and production read-side shutdown
  lifecycle slices
- general mapping of transport byte chunks into sans-I/O input events beyond
  the checked adapter-owned multi-event routing, production multi-chunk
  routing, production two-stream multi-cycle routing, deadline-aware
  lifecycle, cancellable lifecycle, and cancellable deadline-aware lifecycle
  fixtures
- general mapping of outgoing chunks back to host transport writes beyond the
  checked ordered `SendBytes` projection paths in the socket routing,
  owned-lifecycle, deadline-aware lifecycle, cancellable lifecycle, and
  cancellable deadline-aware lifecycle slices, the adapter-owned multi-handler
  ordered `net::write_chunks` projection slice, the production multi-chunk
  routing `net::write_chunks` projection slice, the adapter-owned outbound
  write-failure boundary, the HTTP/2 adapter/core ordered
  `net::write_chunks` projection slice, plus the source-visible ordered
  chunk-list boundary,
  deadline-aware stream-write boundary,
  deadline-aware chunk-list stream-write boundary, cancellable
  deadline-aware stream-write boundary, and cancellable deadline-aware
  chunk-list stream-write boundary
- composed use of `net`, `time`, and `concurrency` effects beyond the checked
  adapter-level cancellable stream routing, receiver-list cancellable
  channel-first routing, receiver-list timeout-result selection, receiver-list
  cancellable timeout-result selection, two-receiver timeout-result selection,
  two-receiver cancellable timeout-result selection, socket/channel routing,
  deadline-aware lifecycle, cancellable lifecycle, cancellable deadline-aware
  lifecycle, clean shutdown lifecycle, and
  multi-handler outbound write-ordering slices
- richer channel-first stream event routing beyond the checked two-route,
  three-route, four-route, general receiver-list routing helper,
  receiver-list timeout,
  receiver-list timeout-result selection,
  receiver-list cancellable timeout-result selection, two-receiver
  timeout-result selection, two-receiver cancellable timeout-result selection,
  and receiver-list cancellable channel-first completion-outcome fixture
  shapes. Additional work should improve routing ownership, lifecycle,
  cancellation, transport integration, or adapter APIs; adding another
  same-shaped route-count fixture is not remaining proposal work.
- richer per-stream task handling beyond the context-based
  `task::spawn_with<Result, Context>` handler boundary. Additional work should
  improve task ownership, lifecycle, cancellation, or adapter APIs, not add
  another same-shaped spawned-handler arity.
- follow-up deadline, timeout, and cancellation adapter APIs only when a
  concrete adapter use case needs capabilities beyond
  `time::monotonic_ms`, `time::timeout_ms`, `time::deadline_after_ms`,
  `time::deadline_at_ms`, `time::wait_until`, `time::cancel_token`,
  `time::cancel`, and
  `time::is_cancelled`, `time::wait_until_cancellable`, plus
  `time::wait_until_cancellable_outcome`, deadline-aware listener accept,
  cancellable deadline-aware listener accept, and deadline-aware and
  cancellable deadline-aware stream read, `time::cancel_owner`,
  `time::cancel_token_from`, `time::cancel_owned`, and
  `time::is_cancelled_owner`
- richer production socket APIs not covered by the checked deterministic
  fixture, source-visible client connect, source-visible listen/connect
  pairing, and loopback adapter shapes

## Discussion Result: Network Effect Labels

Implemented first socket slices are specified by
`../specification/names-effects.md` and `../specification/execution.md`.
Completed fixture-backed listen, accept, client connect, read, write, and
close operations use the existing coarse `net` effect label and remain
runtime boundaries.
Listener endpoint text is current behavior for fixture-backed and
production-loopback listeners through `net::listener_local_addr`; the helper
preserves listener ownership, exposes only a string, and keeps the same coarse
`net` effect boundary.
Accepted-stream and connected-stream endpoint text is current behavior for
fixture-backed streams and production-loopback streams through
`net::stream_local_addr` and `net::stream_peer_addr`; the helpers preserve
stream ownership, expose only strings, and keep the same coarse `net` effect
boundary.
The completed listener endpoint text inspection slice is archived under
`../reference/implemented-proposals/network-listener-address-metadata.md`.
The completed endpoint text inspection slice is archived under
`../reference/implemented-proposals/network-stream-address-metadata.md`.
The completed source-visible client connect slice is archived under
[Network Client Connect Boundary](../reference/implemented-proposals/network-client-connect-boundary.md).
The completed source-visible production listener/client pairing slice is
archived under
[Network Production Listen Connect Lifecycle](../reference/implemented-proposals/network-production-listen-connect-lifecycle.md).
The completed source-visible stream state inspection slice is archived under
[Network Stream State Inspection](../reference/implemented-proposals/network-stream-state-inspection.md).

The remaining transport surface should keep the existing coarse `net` effect
label until effect handlers or an equivalent runtime permission mechanism are
implemented. Listen, connect, accept, read, write, and close operations should be
distinguished by standard-library function names, typed values, and
diagnostics rather than by separate effect labels.

This preserves the current effect-label source surface and avoids requiring the
HTTP/2 design driver to decide a full network permission taxonomy before the
runtime can express, intercept, or delegate those permissions. Fine-grained
labels such as listen-only, accept-only, read-only, write-only, or close-only
network effects are therefore out of scope for this proposal. If later runtime
work needs static separation between network access modes, it should introduce
that split in a follow-up proposal with concrete effect-handler behavior,
compatibility rules, and migration guidance.

## Discussion Result: Channel Byte Views

Channel sends should use the same frozen `ByteView` semantics as other value
crossings. Sending a byte view preserves the bounded byte sequence for the
receiver even if the connection task later drops consumed parser input.

The transport boundary should therefore pass byte chunks and byte views through
typed channel values without introducing a separate channel-only byte slice
type. Runtime implementations may share or copy backing storage, but source
programs should treat the received value as immutable and representation
independent. Stream-routing APIs that retain byte views should carry explicit
size limits because freezing can extend the lifetime of the referenced bytes.

## Discussion Result: Application Handler Boundary

The first server example should expose application behavior as plain handler
functions. Stream tasks and service interfaces should be adapter-level
structures, not the initial application API.

The transport adapter owns sockets, task spawning, channel routing,
flow-control backpressure, and write ordering. It can call a plain handler from
a per-stream task without making every example handler a task entry point or
requiring application code to understand transport scheduling. Fixture tests
can also invoke the same handler directly with decoded stream events and
assert the returned response actions.

A service interface is explicitly outside this proposal's completion
criteria. Routing, deadlines, cancellation, middleware, and resource ownership
need concrete standard-library APIs before that abstraction can be designed
without turning the HTTP/2 design driver into a Web-service framework.

If a future proposal adds a service interface, it should wrap or compose plain
handlers at the adapter layer. It should not replace the event/action handler
boundary chosen here, pass socket handles into application handlers, or make
application code own transport scheduling.

## Discussion Result: Stream Adapter Event Boundary

Implemented first slice: source-level executable examples model decoded stream
work as ordinary event values and response intent as ordinary action values.
A plain handler receives one stream event and explicit application state, then
returns response-action values plus the next state. The same handler is called
directly by a fixture and after an event crosses an existing channel under the
`concurrency` effect. This slice does not add socket ownership, transport
handles, listen/read/write/routing effect labels, or a service interface.

The transport adapter should expose decoded stream work to application code as
plain event values and response-action values, not as transport handles,
connection tasks, or mutable stream objects.

The adapter owns the effectful side of multiplexing: accepting transport
chunks, feeding the sans-I/O core, spawning any per-stream tasks, routing typed
events through channels, preserving frame order, enforcing flow-control
backpressure, and writing encoded output chunks. Application handlers receive
only the stream event value plus explicit application state, then return
response actions and the next application state. Those actions describe
protocol-level intent such as send response bytes, end the stream, reset the
stream, or decline work; they do not write to sockets directly.

This keeps handler examples pure or narrowly effectful even when the adapter is
effectful. A fixture runner can call the same handler with synthesized events,
while a transport adapter can call it from a task and translate response
actions into ordered core transitions. If later service interfaces are added,
they should be adapters over this event/action boundary rather than a
replacement for it.

## Discussion Result: Socket-To-Handler Routing Slice

Implemented narrow slice: an executable specification case composes the
fixture-backed socket boundary with the source-level event/action handler
boundary. Adapter-owned code reads source values, wraps them as ordinary
stream event values, routes those events through existing channel and task
operations under the `concurrency` effect, carries event, state, route, and
trace metadata as one context record through `task::spawn_with<Result, Context>`,
joins the spawned handler task, and translates ordered response actions into
adapter-owned socket output. The handler receives only one ordinary context
value; it does not receive socket handles and does not call `net` functions.

This slice keeps the effect model unchanged. The adapter function composes the
existing `net` and `concurrency` effects because it owns socket I/O, channel
routing, and task spawn/join. The handler boundary remains ordinary source
code and can be called without socket ownership. Non-write response intents
remain values for adapter code to interpret rather than implicit socket
operations.

Implemented clean stream-end slice: an executable specification case uses
`net::read_chunk_or_end` to read one or more chunks, observe clean end as
`None`, translate that clean end into the standard `StreamInput.End` value,
route ordinary stream inputs through an existing channel, call a pure handler,
and project response actions back into ordered `net::write_chunk` calls.
Forced read failure on the same optional read path remains a runtime
transport failure.

Implemented clean listener-end slice: executable specification cases use
`net::accept_or_end` to return `Some(stream)` when the fixture accepts a stream
and `None` when the listener reaches a clean end before accepting. The accepted
stream follows the same stream-handle behavior as `net::accept`. Forced accept
failure on the same optional accept path remains a runtime transport failure.

Implemented deadline-aware listener accept slice: executable specification
cases use `net::accept_until(listener, deadline)` to return `Some(stream)` when
the fixture accepts before the deadline and `None` when the fixture reports
deadline expiry before accepting or the supplied deadline has already expired.
The call infers both `net` and `time` under the existing coarse effect labels.
Forced accept failure on the same optional accept path remains a runtime
transport failure, not a protocol diagnostic.

Implemented cancellable deadline-aware listener accept slice: executable
specification cases use
`net::accept_until_cancellable(listener, deadline, token)` to return
`AcceptStream(stream)` when the fixture accepts before the deadline and before
cancellation, `AcceptEnd` for clean listener end, `AcceptDeadlineExpired` for
fixture-reported or supplied accept deadline expiry, and `AcceptCancelled` for
source-visible token cancellation. The call infers the existing coarse `net`
and `time` effects. Forced accept failure on the same path remains a runtime
transport failure, not a protocol diagnostic.

Implemented deadline-aware stream read slice: executable specification cases
use `net::read_chunk_until(stream, deadline)` to return `Some(bytes)` when the
fixture stream yields a chunk before the deadline and `None` when the fixture
reports deadline expiry before a chunk is read, the supplied deadline has
already expired, or the fixture stream reaches clean end before a chunk is
read. The call infers both `net` and `time` under the existing coarse effect
labels. Forced read failure on the same optional read path remains a runtime
transport failure, not a protocol diagnostic.

Implemented cancellable deadline-aware stream read slice: executable
specification cases use
`net::read_chunk_until_cancellable(stream, deadline, token)` to return
`ReadChunk(bytes)` when a fixture stream yields a chunk before the deadline
and before cancellation, `ReadEnd` for clean stream end,
`ReadDeadlineExpired` for fixture-reported or supplied read deadline expiry,
and `ReadCancelled` for source-visible token cancellation. The call infers
the existing coarse `net` and `time` effects. Forced read failure on the same
path remains a runtime transport failure, not a protocol diagnostic.

Implemented deadline-aware stream write slice: executable specification cases
use `net::write_chunk_until(stream, chunk, deadline)` to return
`WriteCompleted` when a fixture or production-loopback stream writes before
the deadline and `WriteDeadlineExpired` for fixture-reported or supplied
write deadline expiry. The call infers the existing coarse `net` and `time`
effects. Forced write failure on the same path remains a runtime transport
failure, not a protocol diagnostic. The completion record is archived under
`../reference/implemented-proposals/network-write-until-boundary.md`.

Implemented deadline-aware chunk-list stream write slice: executable
specification cases use
`net::write_chunks_until(stream, chunks, deadline)` to write a source-owned
`List<ByteChunk>` in list order and return `WriteCompleted` when the full
list is written before deadline expiry. The same boundary returns
`WriteDeadlineExpired` when deadline expiry wins before the list is fully
written. The call infers the existing coarse `net` and `time` effects.
Forced write failure on the same path remains a runtime transport failure,
not a protocol diagnostic. The completion record is archived under
`../reference/implemented-proposals/network-write-chunks-until-boundary.md`.

Implemented cancellable deadline-aware stream write slice: executable
specification cases use
`net::write_chunk_until_cancellable(stream, chunk, deadline, token)` to
return `WriteCompleted` when a fixture or production-loopback stream writes
before the deadline and before cancellation, `WriteDeadlineExpired` for
fixture-reported or supplied write deadline expiry, and `WriteCancelled` for
source-visible token cancellation. The call infers the existing coarse `net`
and `time` effects. Forced write failure on the same path remains a runtime
transport failure, not a protocol diagnostic. The completion record is
archived under
`../reference/implemented-proposals/network-write-until-cancellable-boundary.md`.

Implemented cancellable deadline-aware chunk-list stream write slice:
executable specification cases use
`net::write_chunks_until_cancellable(stream, chunks, deadline, token)` to
write a source-owned `List<ByteChunk>` in list order and return
`WriteCompleted` when the full list is written before deadline expiry and
before cancellation. The same boundary returns `WriteDeadlineExpired` or
`WriteCancelled` when either outcome wins before the list is fully written.
The call infers the existing coarse `net` and `time` effects. Forced write
failure on the same path remains a runtime transport failure, not a protocol
diagnostic. The completion record is archived under
`../reference/implemented-proposals/network-write-chunks-until-cancellable-boundary.md`.

Implemented deadline-aware accepted-stream lifecycle slice: an executable
specification case accepts a stream with `net::accept_until`, owns that
`NetStream` in adapter code, repeatedly reads with `net::read_chunk_until`
until deadline expiry returns `None`, routes ordinary `StreamInput` values
through an existing channel, calls a pure handler with explicit state, and
projects only ordered `SendBytes` response actions to `net::write_chunk`.
The adapter declares the existing `net`, `time`, and `concurrency` effects;
the handler receives no `NetStream` handle and performs no transport calls.

Implemented cancellable accepted-stream lifecycle slice: an executable
specification case accepts a stream with `net::accept`, owns that `NetStream`
in adapter code, reads input through `net::read_chunk`, routes the ordinary
`StreamInput.Chunk` through an existing channel, observes cancellation through
`time::wait_until_cancellable_outcome`, translates `WaitCancelled` into an
ordinary cleanup response action, and projects only ordered `SendBytes`
response actions to `net::write_chunk`. The adapter declares the existing
`net`, `time`, and `concurrency` effects; the handler receives no `NetStream`
handle and performs no transport, time, or concurrency calls.

Implemented cancellable deadline-aware accepted-stream lifecycle slice: an
executable specification case accepts a stream with
`net::accept_until_cancellable`, owns that `NetStream` in adapter code, reads
input through `net::read_chunk_until_cancellable`, routes ordinary
`StreamInput` values through an existing channel, translates accept and read
clean-end, deadline, and cancellation outcomes into adapter decisions or
ordinary response actions, and projects only ordered `SendBytes` response
actions to `net::write_chunk`. The adapter declares the existing `net`,
`time`, and `concurrency` effects; the handler receives no `NetStream` handle
and performs no transport, time, or concurrency calls.

Implemented adapter-owned clean shutdown slice: an executable specification
case accepts a stream with `net::accept_until_cancellable`, owns the
`NetListener` and `NetStream` in adapter code, routes an ordinary
`StreamInput` value through an existing channel, observes cancellation and
deadline expiry through `time::wait_until_cancellable_outcome`, translates
those outcomes into ordinary response actions, projects only ordered
`SendBytes` actions to `net::write_chunk`, and then explicitly calls
`net::close_stream` followed by `net::close_listener`. The matching
effect-checking case requires `net`, `time`, and `concurrency` at the adapter
boundary while keeping the handler free of transport, time, and concurrency
effects.

Implemented outgoing chunk-list write slice: executable specification cases
use `net::write_chunks(stream, chunks)` to write a source-owned
`List<ByteChunk>` to a `NetStream` in list order under the existing coarse
`net` effect. The call uses the same stream write path and transport-failure
surface as `net::write_chunk`, and pure protocol handlers remain free of
`net` calls.

Implemented adapter-owned outbound write-ordering slice: an executable
specification case accepts deterministic production-loopback streams, reads
ordinary `StreamInput` values through adapter-owned socket calls, routes those
values through an existing channel, calls multiple pure handler functions that
receive no `NetStream` handles, combines their ordinary `ResponseAction`
values into one explicit adapter-owned order, projects only ordered
`SendBytes` actions to a `List<ByteChunk>`, and writes that list through
`net::write_chunks`. The matching effect-checking case keeps the handlers
transport-free and requires the adapter boundary to declare the existing
`net` and `concurrency` effects.

Implemented adapter-owned outbound write-failure slice: executable
specification cases force the same ordered `net::write_chunks` projection path
to fail after production accept, stream read, ordinary channel routing, and
pure handler response projection. The failure remains a runtime transport
failure owned by the adapter boundary; it does not become a protocol, schema,
codec, or handler failure, and no response write or stream close is recorded.

Implemented production multi-chunk event routing slice: an executable
specification case accepts one deterministic production-loopback stream, reads
multiple configured input chunks through `net::read_chunk_or_end`, converts
each read chunk into an ordinary `StreamInput.Chunk`, routes those values
through an existing channel to a pure handler, observes clean end as
`StreamInput.End`, and projects only ordered `SendBytes` actions back to the
stream through `net::write_chunks`. The adapter owns `net` and `concurrency`;
the handler receives no `NetStream` and calls no transport functions. A
companion task-context case routes each accepted stream event through the same
channel boundary and then through an adapter-owned task helper using
`task::spawn_with<Result, Context>`. The helper carries adapter-owned route and
trace metadata, preserves event sequence before ordered `net::write_chunks`
projection, and calls a pure handler that receives no `NetStream`. A matching
read-failure case forces the same multi-chunk adapter path to fail as a runtime
transport failure after production accept and before any chunk routing,
response writes, stream close,
or clean listener end is recorded.

Implemented per-stream task handler-failure lifecycle slice: an executable
specification case accepts one deterministic production-loopback stream,
routes a `StreamInput.Chunk` through the existing channel and
`task::spawn_with<Result, Context>` helper boundary, observes the handler
returning `Err` as an ordinary source-visible adapter action, skips later
response-byte projection for that failed stream, closes the accepted stream,
and then observes clean listener end. The case records no response write for
the failed stream and keeps host transport failures separate from
handler-owned failure outcomes.

Implemented production two-stream multi-cycle routing slice: an executable
specification case accepts two deterministic production-loopback streams from
one listener, reads multiple chunks from each stream, routes the ordinary
stream values through the same handler/action boundary, writes only
adapter-owned response chunks through `net::write_chunks`, closes each stream,
and observes clean listener end. The adapter owns `net` and `concurrency`; the
handler receives no `NetStream` and calls no transport functions.

Implemented production owner-drain cancellable deadline lifecycle slice: an
executable specification case creates a `CancelOwner` in adapter code, passes
only observer `CancelToken` values to cancellable deadline-aware accept/read
and channel-routing code, drains deterministic production-loopback streams
until clean listener end, calls a pure handler with ordinary `StreamInput`
values and explicit state, projects only ordered `SendBytes` actions through
`net::write_chunks`, closes owned streams and the listener in cleanup, and
checks accept cancellation as an ordinary adapter outcome. The same run case
also accepts one production stream, routes one ordinary stream event through
the channel and task handler boundary, requests owner cancellation, and
observes the next cancellable read as ordinary `ReadCancelled` before another
handler route continues. The matching effect-checking case requires `net`,
`time`, and `concurrency` at the adapter boundary while keeping the handler
free of transport, time, and concurrency effects.

The adapter-owned listener-to-clean-stream-end lifecycle slice is recorded as
implemented in
`../reference/implemented-proposals/network-adapter-ownership-boundary.md`.

The explicit stream close lifecycle slice is recorded as implemented in
`../reference/implemented-proposals/network-stream-close-boundary.md`.

The write-side stream half-close lifecycle slice is recorded as implemented in
`../reference/implemented-proposals/network-stream-shutdown-write-boundary.md`.

The read-side stream half-close lifecycle slice is recorded as implemented in
`../reference/implemented-proposals/network-stream-shutdown-read-boundary.md`.

The source-visible ordered chunk-list write slice is recorded as implemented
in `../reference/implemented-proposals/network-write-chunks-boundary.md`.

The source-visible deadline-aware chunk-list write slice is recorded as
implemented in
`../reference/implemented-proposals/network-write-chunks-until-boundary.md`.

The source-visible cancellable deadline-aware chunk-list write slice is
recorded as implemented in
`../reference/implemented-proposals/network-write-chunks-until-cancellable-boundary.md`.

The explicit listener-close boundary is recorded as implemented in
`../reference/implemented-proposals/network-listener-close-boundary.md`.

The source-visible production listen/connect lifecycle slice is recorded as
implemented in
`../reference/implemented-proposals/network-production-listen-connect-lifecycle.md`.

The adapter-owned multi-handler outbound write-ordering and outbound
write-failure slices are recorded as implemented in
`../reference/implemented-proposals/network-adapter-outbound-write-ordering.md`.

The adapter-owned clean shutdown slice is recorded as implemented in
`../reference/implemented-proposals/network-adapter-clean-shutdown.md`.

The production-loopback listen, sequential accept, read, write, clean listener
end, close lifecycle, two-stream adapter handler/action lifecycle,
listener-drain adapter lifecycle, listener-drain read-failure runtime
boundary, deadline-aware adapter lifecycle, deadline-aware accept-failure
runtime boundary, deadline-aware read-failure runtime boundary, and focused
adapter close-failure runtime boundary slices are recorded as implemented in
`../reference/implemented-proposals/network-production-loopback-lifecycle.md`.

The production-loopback cancellable deadline-aware adapter lifecycle and
accept/read outcome boundary slice is recorded as implemented in
`../reference/implemented-proposals/network-production-cancellable-deadline-lifecycle.md`.

The production owner-drain cancellable deadline lifecycle slice is recorded
as implemented in
`../reference/implemented-proposals/network-production-owner-drain-lifecycle.md`.

The production multi-chunk event routing slice is recorded as implemented in
`../reference/implemented-proposals/network-production-multi-chunk-routing.md`,
including runtime success, task-context trace/order, handler-failure
lifecycle, read-failure, and static effect-boundary evidence.

The standard stream adapter routing helper slice is recorded as implemented
in
`../reference/implemented-proposals/network-stream-adapter-routing-helper.md`.

The production two-stream multi-cycle routing slice is recorded as implemented
in
`../reference/implemented-proposals/network-production-two-stream-multi-cycle-routing.md`.

The source-visible stream state inspection slice is recorded as implemented
in
`../reference/implemented-proposals/network-stream-state-inspection.md`.

The HTTP/2 adapter/core write boundary slice is recorded as implemented in
`../reference/implemented-proposals/network-http2-adapter-core-write-boundary.md`.

The receiver-list select-many, timeout, timeout-result, and cancellable
timeout-result channel-first stream routing slices are recorded as implemented
in
`../reference/implemented-proposals/network-channel-select-many-routing.md`.
They include the general receiver-list routing helper example, cleanup of the
stale five-route through thirty-route fixture series,
`channel::select_many_priority`, `channel::select_many_timeout`,
`channel::select_many_timeout_result`, and
`channel::select_many_timeout_cancellable`.
The cleanup leaves the two-, three-, four-route, and general receiver-list
fixtures as the canonical checked routing shapes; further same-shaped
route-count fixtures are not proposal work.

The two-receiver cancellable timeout-result selection slice, including
`channel::select_timeout_cancellable`, is recorded as implemented in
`../reference/implemented-proposals/network-channel-select-timeout-cancellable.md`.

The two-receiver timeout-result selection slice, including
`channel::select_timeout_result`, is recorded as implemented in
`../reference/implemented-proposals/network-channel-select-timeout-result.md`.

The context-based stream-task slice is now part of the current task spawning
specification instead of a planned arity-growth path.
The proposal no longer treats more `spawn_withN` or positional handler
arguments as remaining work; handler context should be carried as one ordinary
source value unless a future proposal introduces a different task API.

## Discussion Result: Transport Error Boundary

Implemented first slices: descriptor-backed `net::receive_chunk` reports
malformed host-fed `VELN_NET_CHUNK_HEX` as a runtime failure,
descriptor-backed `net::send_chunk` reports failed outgoing event recording as
a runtime failure, and fixture-backed socket listen, accept, read, and write
failures are runtime failures. Clean end observed through
`net::accept_or_end` or `net::read_chunk_or_end` is a successful
adapter-observable condition, not a runtime failure. Successful
descriptor-backed `net` and `time` calls plus
fixture-backed listener/stream calls remain current behavior under
`../specification/names-effects.md`.

Transport failures should enter the system as host or runtime errors owned by
the transport adapter. Socket read failures, write failures, accept failures,
deadline expiry, cancellation, and local resource exhaustion are not peer
protocol errors by themselves because they do not prove that the remote peer
violated the protocol.

The adapter may translate a clean end-of-stream into the pure core's
`StreamInput` end event. If the core was waiting for more bytes, the core can
then produce a truncation diagnostic for the incomplete protocol fact. The
underlying transport condition remains related host context when reported; it
does not replace the schema or protocol diagnostic that names the failed byte
or state fact.

Protocol errors remain ordinary protocol ADTs produced by the pure core from
decoded bytes, frame ordering, stream state, settings, and configured limits.
The transport adapter decides how to act on those values by closing a
connection, resetting a stream, emitting response frames, or reporting a
diagnostic. Failed writes while taking that action stay transport failures,
not new protocol errors.

This keeps blame precise in tests and diagnostics: peer input failures are
reported through schema, codec, or protocol diagnostics, while unavailable
I/O, timeouts, and host capacity problems are reported through the runtime or
adapter surface with related connection and stream context when available.

## Discussion Result: Deadline And Timeout API

Implemented first slices: `time::timeout_ms(milliseconds)`,
`time::deadline_after_ms(milliseconds)`, `time::wait_until(deadline)`,
`time::deadline_at_ms(target_ms)`, `time::monotonic_ms()`,
`time::cancel_token()`, `time::cancel(token)`, and
`time::wait_until_cancellable(deadline, token)` use the existing `time` effect
label at the runtime boundary. `time::monotonic_ms()` returns a host-owned
monotonic millisecond counter for elapsed-time measurement without exposing a
wall-clock date API. `time::deadline_after_ms` creates a `Deadline` from a
relative millisecond duration, and `time::deadline_at_ms` creates the same
source-visible `Deadline` shape from an absolute monotonic millisecond target.
Existing deadline-aware socket, wait, and adapter calls consume both
construction paths without separate source code paths. `CancelToken` is the
first source-visible cancellation handle for adapter-owned waits.
`time::is_cancelled(token)` observes that handle as `Bool` without waiting or
requesting cancellation.
`time::wait_until_cancellable_outcome(deadline, token)` returns
`CancellableWaitOutcome` so adapter code can translate completed waits,
deadline expiry, and cancellation into ordinary source decisions. Executable
stream adapter cases compose that outcome with channel-routed `StreamInput`
values and ordinary response action values for adapter-owned wait decisions.
The receiver-list cancellable channel-first fixture instead routes ordinary
`StreamInput` values through `channel::select_many_timeout_cancellable`, then
maps routed, timed-out, and cancelled selection results into ordinary adapter
completion values and then action values without adding another fixed
route-count fixture. The
receiver-list cancellable timeout-result helper
`channel::select_many_timeout_cancellable` combines receiver-list priority,
timeout, and `CancelToken` observation in one `channel` boundary that returns
`Ok(Some(selected))`, `Ok(None)`, or `Err(SelectError)`. Host fixtures can force
timeout expiry, deadline expiry, or cancellable-wait cancellation as runtime
failures through the runtime-failure wait. These calls do not add a separate
richer timer effect or timer-specific source construct.

Implemented cancellation-owner slice: executable specification cases use
`time::cancel_owner` to create an adapter-owned cancellation owner,
`time::cancel_token_from` to expose only an observer `CancelToken` to routing,
wait, and cancellable socket-read code, and `time::cancel_owned` to request
cancellation during adapter cleanup. After owner-requested cancellation,
`time::wait_until_cancellable_outcome` returns `WaitCancelled` and
`net::read_chunk_until_cancellable` returns `ReadCancelled` as ordinary
adapter-observable values. The calls infer the existing coarse `time` effect,
and the adapter example composes them with existing `net` and `concurrency`
boundaries while leaving the handler free of transport effects. Owner-derived
observer tokens reject direct `time::cancel(token)` at the runtime boundary;
direct tokens from `time::cancel_token` keep the existing compatibility path.
The completion record is archived under
`../reference/implemented-proposals/network-cancel-owner-boundary.md`.
Implemented cancellation-owner status slice: executable specification cases
use `time::is_cancelled_owner(owner)` to inspect a `CancelOwner` directly
under the existing coarse `time` effect. The focused run case observes
`false`, calls `time::cancel_owned(owner)`, and then observes `true` without
creating an observer token or changing `time::is_cancelled(token)`. The
matching effect case requires public callers to declare `time`. The completion
record is archived under
`../reference/implemented-proposals/network-cancel-owner-status.md`.
The monotonic clock completion record is archived under
`../reference/implemented-proposals/network-monotonic-clock-boundary.md`.
The absolute monotonic deadline completion record is archived under
`../reference/implemented-proposals/network-deadline-at-boundary.md`.

The transport adapter should own wall-clock interaction. It can compute
deadlines, wait for timeouts, cancel pending transport work through a
source-visible handle, and translate deadline expiry or cancellation into
transport-layer outcomes while carrying the `time` effect. The pure sans-I/O
core continues to receive explicit input events and protocol state values; it
does not read time, sleep, or observe host timers or cancellation handles.

This keeps the first integration boundary aligned with the current coarse
effect model. A later runtime proposal may add richer timer handles,
cancellation-owner capabilities beyond the current owner/token/status split, or
scheduler APIs if examples need them, but that work should extend the `time`
standard-library surface rather than introduce deadline behavior into schemas
or the pure protocol core.

The current proposal is complete at the owner/token/status/outcome boundary.
It should not add richer timer handles, scheduler APIs, or cancellation
capabilities without a concrete adapter use case. Future work that needs those
capabilities should open a separate runtime proposal that extends the `time`
standard-library surface.

## Non-Goals

- Do not block the sans-I/O protocol core on sockets.
- Do not require TLS or ALPN.
- Do not change current implemented effect labels until runtime APIs require
  it.
- Do not define HTTP application routing.

## Remaining Completion Criteria

- Specification work distinguishes pure protocol functions from transport
  effectful adapter functions.
- The checked production-loopback lifecycle, including the two-stream
  listener sequence, two-stream adapter handler/action boundary,
  listener-drain lifecycle, listener-drain read-failure runtime boundary,
  deadline-aware adapter lifecycle, deadline-aware accept and read failure
  runtime boundaries, adapter-owned clean shutdown lifecycle,
  adapter-owned cancellation owner lifecycle, and clean listener end, remains
  current evidence for
  deterministic host-owned loopback streams. Remaining examples still need
  richer production adapter socket ownership beyond the checked
  fixture-backed listener/stream handles, stream-task handler, clean
  stream-end, optional accept, deadline-aware optional accept, adapter-owned
  lifecycle, channel-first routing, cancellable routing, accepted-stream
  lifecycle, cancellable channel-first completion, and source-visible
  listener/client pairing slices; remaining examples still need richer
  production socket APIs. Deadline and
  cancellation behavior is complete for this proposal at the current relative
  and absolute monotonic `Deadline`, `CancelToken`, cancellation status-query,
  cancellable wait-outcome, cancellable deadline-aware listener accept, stream
  read, stream write, and accepted-stream lifecycle boundaries, plus the
  current cancellation owner/token/status split.
- Effect inference and diagnostics cover any new compiler-known network,
  timer, channel, or task calls introduced by the remaining adapter work.
- The HTTP/2 design driver can remain pure while leaving a documented route to
  transport integration.
