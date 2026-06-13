# Network Effect Integration Boundary

Status: proposed

This proposal tracks remaining work between a pure sans-I/O protocol core and
transport integration. The first descriptor-backed `net` and `time`
boundary calls are current behavior under
`../specification/names-effects.md`, including host-runtime failures for
malformed received bytes, failed outgoing event recording, and forced timeout
expiry. This page keeps the larger transport adapter, routing, deadline API,
cancellation, and socket work open.

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
boundary calls for:

- socket listen, accept, read, and write operations
- mapping transport byte chunks into sans-I/O input events
- mapping outgoing chunks back to host transport writes
- composed use of `net`, `time`, and `concurrency` effects
- channel-first stream event routing
- per-stream task handling
- richer deadline, timeout, and cancellation adapter APIs beyond
  `time::timeout_ms`
- ownership of frame ordering, flow control, and transport writes

## Discussion Result: Network Effect Labels

The remaining transport surface should keep the existing coarse `net` effect
label. Listen, accept, read, and write operations should be distinguished by
standard-library function names, typed values, and diagnostics rather than by
separate effect labels.

This preserves the current effect-label source surface and avoids requiring the
HTTP/2 design driver to decide a full network permission taxonomy before the
socket API exists. If later runtime work needs static separation between
network access modes, it should introduce that split with concrete APIs,
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

## Discussion Result: Transport Error Boundary

Implemented first slice: descriptor-backed `net::receive_chunk` reports
malformed host-fed `VELN_NET_CHUNK_HEX` as a runtime failure, and
descriptor-backed `net::send_chunk` reports failed outgoing event recording as
a runtime failure. Successful descriptor-backed `net` and `time` calls remain
current behavior under `../specification/names-effects.md`.

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

Implemented first slice: `time::timeout_ms(milliseconds)` uses the existing
`time` effect label, waits at the runtime boundary, and can be forced by a
host fixture to report timeout expiry as a runtime failure. It does not add a
separate richer timer effect or timer-specific source construct.

The transport adapter should own wall-clock interaction. It can compute
deadlines, wait for timeouts, cancel pending transport work, and translate
deadline expiry into transport-layer outcomes while carrying the `time` effect.
The pure sans-I/O core continues to receive explicit input events and protocol
state values; it does not read time, sleep, or observe host timers.

This keeps the first integration boundary aligned with the current coarse
effect model. A later runtime proposal may add concrete timer handles,
monotonic-clock values, cancellation tokens, or scheduler APIs if examples need
them, but that work should extend the `time` standard-library surface rather
than introduce deadline behavior into schemas or the pure protocol core.

## Non-Goals

- Do not block the sans-I/O protocol core on sockets.
- Do not require TLS or ALPN.
- Do not change current implemented effect labels until runtime APIs require
  it.
- Do not define HTTP application routing.

## Remaining Completion Criteria

- Specification work distinguishes pure protocol functions from transport
  effectful adapter functions.
- Examples show adapter-owned socket reads, writes, stream routing, richer
  deadline APIs, and cancellation once those runtime APIs exist.
- Effect inference and diagnostics cover any new compiler-known network,
  timer, channel, or task calls introduced by the remaining adapter work.
- The HTTP/2 design driver can remain pure while leaving a documented route to
  transport integration.
