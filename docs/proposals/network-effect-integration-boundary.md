# Network Effect Integration Boundary

Status: proposed

This proposal tracks remaining work between a pure sans-I/O protocol core and
transport integration. Current implemented transport, channel, task, deadline,
and cancellation slices are specified by `../specification/names-effects.md`
and `../specification/execution.md`; completed proposal records live under
`../reference/implemented-proposals/`. This page keeps the larger transport
adapter, richer stream routing, richer deadline, cancellation, and socket work
open.

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
stream routing slices, checked channel-first route-count slices, checked task
slices, and narrow deadline and cancellation slices, for:

- production socket ownership and lifecycle beyond the fixture-backed listen,
  optional accept, deadline-aware optional accept, optional stream-read,
  deadline-aware optional stream-read, cancellable deadline-aware stream-read,
  ordered write lifecycle slice, and checked adapter-owned
  listener-to-clean-stream-end, deadline-aware accepted-stream lifecycle,
  cancellable accepted-stream lifecycle, cancellable deadline-aware
  accepted-stream lifecycle, and explicit stream close lifecycle slices
- general mapping of transport byte chunks into sans-I/O input events beyond
  the checked adapter-owned multi-event routing, deadline-aware lifecycle,
  cancellable lifecycle, and cancellable deadline-aware lifecycle fixtures
- general mapping of outgoing chunks back to host transport writes beyond the
  checked ordered `SendBytes` projection paths in the socket routing,
  owned-lifecycle, deadline-aware lifecycle, cancellable lifecycle, and
  cancellable deadline-aware lifecycle slices
- composed use of `net`, `time`, and `concurrency` effects beyond the checked
  adapter-level cancellable stream routing, receiver-list cancellable
  channel-first routing, receiver-list timeout-result selection, receiver-list
  cancellable timeout-result selection, two-receiver cancellable
  timeout-result selection, socket/channel routing, deadline-aware lifecycle,
  cancellable lifecycle, and cancellable deadline-aware lifecycle slices
- richer channel-first stream event routing beyond the checked two-route,
  three-route, four-route, receiver-list select-many route-count fixtures,
  general receiver-list routing helper, receiver-list timeout,
  receiver-list timeout-result selection,
  receiver-list cancellable timeout-result selection, two-receiver
  cancellable timeout-result selection, and receiver-list cancellable
  channel-first fixture shapes. Additional work should improve routing
  ownership, lifecycle, cancellation, transport integration, or adapter APIs;
  adding another same-shaped route-count fixture is not remaining proposal
  work.
- richer per-stream task handling beyond the context-based
  `task::spawn_with<Result, Context>` handler boundary. Additional work should
  improve task ownership, lifecycle, cancellation, or adapter APIs, not add
  another same-shaped spawned-handler arity.
- richer deadline, timeout, and cancellation adapter APIs beyond
  `time::timeout_ms`, `time::deadline_after_ms`, `time::wait_until`,
  `time::cancel_token`, `time::cancel`, and
  `time::is_cancelled`, `time::wait_until_cancellable`, plus
  `time::wait_until_cancellable_outcome`, deadline-aware listener accept,
  cancellable deadline-aware listener accept, and deadline-aware and
  cancellable deadline-aware stream read
- ownership of frame ordering, flow control, and transport writes

## Discussion Result: Network Effect Labels

Implemented first socket slices are specified by
`../specification/names-effects.md` and `../specification/execution.md`.
Completed fixture-backed listen, accept, read, write, and close operations use
the existing coarse `net` effect label and remain runtime boundaries.

The remaining transport surface should keep the existing coarse `net` effect
label. Listen, accept, read, write, and close operations should be
distinguished by standard-library function names, typed values, and
diagnostics rather than by separate effect labels.

This preserves the current effect-label source surface and avoids requiring the
HTTP/2 design driver to decide a full network permission taxonomy from the
first fixture-backed socket calls. If later runtime work needs static
separation between network access modes, it should introduce that split with
concrete APIs, compatibility rules, and migration guidance.

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

A service interface is deferred until routing, deadlines, cancellation,
middleware, and resource ownership have concrete standard-library APIs. That
future interface should wrap or compose plain handlers rather than forcing the
HTTP/2 design driver to define a Web-service framework.

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

The adapter-owned listener-to-clean-stream-end lifecycle slice is recorded as
implemented in
`../reference/implemented-proposals/network-adapter-ownership-boundary.md`.

The explicit stream close lifecycle slice is recorded as implemented in
`../reference/implemented-proposals/network-stream-close-boundary.md`.

The bounded receiver-list select-many, timeout, timeout-result, and
cancellable timeout-result channel-first stream routing slices, including the
general receiver-list routing helper example, the
`channel::select_many_priority` and `channel::select_many_timeout` helpers plus
`channel::select_many_timeout_result` and
`channel::select_many_timeout_cancellable`, are recorded as implemented in
`../reference/implemented-proposals/network-channel-select-many-routing.md`.

The two-receiver cancellable timeout-result selection slice, including
`channel::select_timeout_cancellable`, is recorded as implemented in
`../reference/implemented-proposals/network-channel-select-timeout-cancellable.md`.

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
`time::cancel_token()`, `time::cancel(token)`, and
`time::wait_until_cancellable(deadline, token)` use the existing `time` effect
label and wait at the runtime boundary. `CancelToken` is the first
source-visible cancellation handle for adapter-owned waits.
`time::is_cancelled(token)` observes that handle as `Bool` without waiting or
requesting cancellation.
`time::wait_until_cancellable_outcome(deadline, token)` returns
`CancellableWaitOutcome` so adapter code can translate completed waits,
deadline expiry, and cancellation into ordinary source decisions. Executable
stream adapter cases compose that outcome with channel-routed `StreamInput`
values and ordinary response action values: one fixture output shows completed
waits keeping handler-produced actions, deadline expiry becoming a retry
action, and cancellation becoming a cleanup action. The receiver-list
cancellable channel-first fixture routes ordinary `StreamInput` values through
`channel::select_many_timeout` before applying the same wait-outcome
translation. The receiver-list cancellable timeout-result helper
`channel::select_many_timeout_cancellable` combines receiver-list priority,
timeout, and `CancelToken` observation in one `channel` boundary that returns
`Ok(Some(selected))`, `Ok(None)`, or `Err(SelectError)`. Host fixtures can force
timeout expiry, deadline expiry, or cancellable-wait cancellation as runtime
failures through the runtime-failure wait. These calls do not add a separate
richer timer effect or timer-specific source construct.

The transport adapter should own wall-clock interaction. It can compute
deadlines, wait for timeouts, cancel pending transport work through a
source-visible handle, and translate deadline expiry or cancellation into
transport-layer outcomes while carrying the `time` effect. The pure sans-I/O
core continues to receive explicit input events and protocol state values; it
does not read time, sleep, or observe host timers or cancellation handles.

This keeps the first integration boundary aligned with the current coarse
effect model. A later runtime proposal may add richer timer handles,
monotonic-clock values, cancellation ownership APIs beyond `CancelToken`, or
scheduler APIs if examples need them, but that work should extend the `time`
standard-library surface rather than introduce deadline behavior into schemas
or the pure protocol core.

## Non-Goals

- Do not block the sans-I/O protocol core on sockets.
- Do not require TLS or ALPN.
- Do not change current implemented effect labels until runtime APIs require
  it.
- Do not define HTTP application routing.

## Remaining Completion Criteria

- Specification work distinguishes pure protocol functions from transport
  effectful adapter functions.
- Examples show production adapter socket ownership beyond the first
  fixture-backed listener/stream handles, narrow multi-event
  socket-to-handler routing, stream-task handler, clean stream-end, optional
  accept, deadline-aware optional accept, adapter-owned lifecycle, two-route,
  three-route, four-route, receiver-list select-many routing through the
  current checked boundary, receiver-list timeout,
  receiver-list timeout-result selection, receiver-list cancellable
  timeout-result selection, two-receiver cancellable timeout-result selection,
  and receiver-list cancellable channel-first stream routing, deadline-aware
  accepted-stream lifecycle, cancellable accepted-stream lifecycle,
  cancellable deadline-aware accepted-stream lifecycle, context-based spawned
  handler task, adapter-level cancellable stream routing, and the general
  receiver-list stream routing abstraction that replaces route-count fixture
  growth;
  remaining examples still need production lifecycle coverage and richer
  deadline and cancellation APIs beyond the current relative `Deadline`,
  `CancelToken`, cancellation status-query, cancellable wait-outcome,
  cancellable deadline-aware listener accept, stream read, and accepted-stream
  lifecycle boundaries.
- Effect inference and diagnostics cover any new compiler-known network,
  timer, channel, or task calls introduced by the remaining adapter work.
- The HTTP/2 design driver can remain pure while leaving a documented route to
  transport integration.
