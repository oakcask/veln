---
role: proposal
update-when: The planned standard-library network system handler, host-error translation, stream-resource lifecycle, write-all helper, duplex transport adapter, or lexical-cleanup prerequisite changes.
---

# Standard-library networking and its effect boundary

## Outcome

The standard package now exports the `net` foundation described by the current
[standard-library networking specification](../specification/standard-library-networking.md).
The public `net::IO` effect and direct stream-operation facade are now current
behavior. The remaining work will add the portable system implementation. An
application will select that host implementation by handling `net::IO` with
`net::system()`.

This keeps two concerns separate:

- `net::IO` is the substitutable contract used by application and protocol
  code.
- The existing host `net` and `time` effects will be the trusted runtime
  boundary used by the proposed system handler.

The remaining delivery covers host-backed TCP streams, resolution, deadlines,
cancellation, cleanup, `write_all`, and the duplex transport adapter. It does
not attempt full API parity with another language's network library.

## Motivation

Veln started with host-backed socket operations and opaque listener and stream
values exposed only as compiler-known symbols. The exported standard-package
module now owns the public operation contract and substitution boundary. The
remaining work moves host implementation, failure translation, and resource
state behind that boundary.

Go's `net` package supplies a useful division of responsibility: dialing,
listening, accepting, address inspection, name resolution, and connection I/O
belong to one portable package, while protocol libraries build above its
connection abstraction. Veln should use that division without copying Go's
method model or mutable deadline API. The source references are the
[Go `net` package overview](https://pkg.go.dev/net) and the
[Go connection contracts](https://go.dev/src/net/net.go).

Veln also needs a visible answer to a question that Go does not have: which
effect represents network access, and where can a program replace its
implementation? The current public `net::IO` effect gives library authors one
effect to declare and supplies the substitution boundary. The remaining work
will add the standard host handler and deterministic conformance
infrastructure at that boundary.

## Goals

- Back the current portable stream API with system handling for TCP, TCP over
  IPv4, and TCP over IPv6.
- Return typed ordinary failures instead of turning expected operating-system
  outcomes into runtime diagnostics.
- Supply the explicit system handler for the replaceable DNS and socket
  operations.
- Preserve precise end-of-stream, deadline, cancellation, partial-write, and
  close outcomes.
- Keep protocol code independent of host socket handles through
  `transport::DuplexStream` when it needs only duplex byte transport.
- Provide executable acceptance evidence for the system handler and for a
  deterministic test handler.

## Non-goals

The remaining proposal does not include:

- UDP, raw IP, multicast, or packet-oriented sockets;
- Unix-domain sockets;
- network-interface enumeration or routing configuration;
- TLS, HTTP, proxies, or application protocols;
- file-descriptor conversion or platform-specific socket options;
- a stable ABI for the private host intrinsics;
- transparent retries after an operation has returned a failure.

These require separate proposals because they introduce different resource,
message-boundary, portability, or security contracts. In particular, a future
packet API must not encode datagrams as streams.

## Remaining operation semantics

The public resource aliases, outcome values, complete `net::IO` effect, and
direct forwarding facade are current behavior in the
[standard-library networking specification](../specification/standard-library-networking.md).
The following semantics are requirements for the planned system handler, not
guarantees of the current direct forwarding facade. The remaining work also
adds `write_all`:

```veln
pub fn write_all(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<(), NetError> effects [net::IO]
```
Under the proposed system handler, an absent deadline will mean that elapsed
time does not end the operation. An absent cancellation token will mean that
cancellation does not end the operation. If both are present and observable
before success, the first condition observed by the handler must determine
`TimedOut` or `Cancelled`. An operation may still return another host failure
that was already committed before either condition was observed.

The system handler's `resolve` must preserve its preferred endpoint order and
remove exact duplicates. It must return `NameNotFound` when no endpoint is
available. Its `connect` may try resolved endpoints, but it must return only
one stream or one error.

The system handler must return `ReadEnd` only after the peer's write half has
ended and all previously received bytes have been returned. It must not use
an empty chunk as an end marker. The current facade forwards any handler's
`ReadOutcome` unchanged, including an empty `ReadChunk`.

The system handler's `write` must report the count committed to the stream.
`Written` may contain a count smaller than the input length. The caller may
retry only the uncommitted suffix. `WriteFailed` must report both the committed
prefix and the failure. The caller must process the committed count before
handling the error and must not retry any committed bytes.

The proposed `write_all` must repeat the write operation for the uncommitted
suffix until all bytes are written or an error occurs. It must stop after
`WriteFailed`, including when that outcome committed a non-empty prefix. A
zero-byte `Written` outcome before completion must become `NetError` with kind
`Other`; this prevents an unbounded retry loop.

## Handler model

### System handler

The remaining implementation will export:

```veln
pub handler system() for net::IO effects [net, time]
```

The handler will translate portable operations to private host intrinsics.
Those intrinsics will be compiler/runtime implementation details, not
source-resolvable package APIs. Handling `net::IO` with the proposed handler
will remove that effect and introduce only the host `net` and `time` effects.

The proposed handler has one fixed effect set, so an untimed operation will
also introduce host `time`. This contract accepts that conservative effect because
splitting timed and untimed operations across handlers would split ownership
of the same resources. A later change may remove the extra host effect only if
handler effect inference can do so without changing the `net::IO` API.

With the proposed `write_all` available, reusable code could declare `net::IO`:

```veln
use net from "std"

fn serve_once(address: net::Address) -> Result<(), net::NetError> effects [net::IO]
	let listener = net::listen(address)?
	let stream = net::accept(listener)?
	let incoming = net::read(stream)?
	let result = match incoming
		net::ReadChunk(bytes) => net::write_all(stream, bytes, None, None)
		net::ReadEnd => Ok(())
	end
	net::close_stream(stream)?
	net::close_listener(listener)?
	result
end
```

Once `net::system()` exists, the application could choose the host boundary:

```veln
pub fn main() -> Result<(), net::NetError> effects [net, time]
	handle serve_once(net::Address(net::Tcp, "127.0.0.1", 8080)) with net::system()
end
```

The current facade does not install any handler implicitly. Once the system
handler exists, an entry point must apply it or a caller must propagate
`net::IO`. This keeps network authority visible during composition and makes
missing-handler behavior a static effect error.

### Test handlers

The runtime conformance harness will need a deterministic `net::IO` handler.
Like the proposed system handler, it must be trusted to create opaque
`Listener` and `Stream` references. This handler will be test infrastructure,
not an exported standard-package module. A source-defined handler can deny,
trace, or delegate operations, but it cannot fabricate a successful resource
reference through a public constructor.

A deterministic handler must be able to script:

- resolution results and failures;
- incoming connections and accepted stream identities;
- read chunks, end-of-stream, and read failures;
- partial writes and write failures;
- deadline and cancellation outcomes;
- the local and peer endpoints of each resource.

The planned handler must record operations in call order. The host-side
conformance harness will inspect that record to verify cleanup and retry
behavior. The planned script and trace formats will be repository-internal
test data, not public Veln APIs.

### Duplex transport adapter

`transport::DuplexStream` is the existing narrow effect for a protocol that
already owns one connected stream. The existing
`transport::net::net_stream(stream)` adapter uses the coarse host `net` effect
through `net::read_chunk_or_end` and `net::write_chunks`. It does not use the
current `net::IO` boundary or the proposed `net::write_all` helper.

The standard library will add an adapter handler that captures a `net::Stream`,
performs `net::read` through the current `net::IO` boundary, uses the proposed
`net::write_all` helper, and maps `NetError` into the transport failure type
selected by the transport contract.

The proposed adapter must not listen, accept, resolve, connect, or close the
captured stream. The caller will retain lifecycle ownership. This prevents a
protocol handler from silently closing a stream that another layer intends to
reuse.

## Resource lifecycle

The proposed system handler will own resource state. Its operations on
different resources may proceed concurrently. For one stream, at most one read
and one write may be in flight; one read and one write may proceed
concurrently. A second concurrent read or a second concurrent write must
return `Busy` until the active operation finishes. This avoids an unspecified
byte split between callers.

### Listener transitions

| Current state | Operation and outcome | Next state | Observable result |
| --- | --- | --- | --- |
| Open | `accept` succeeds | Open | A fresh open `Stream` |
| Open | `accept` times out or is cancelled | Open | `TimedOut` or `Cancelled` |
| Open | `close_listener` | Closed | `Ok(())`; blocked accepts return `Closed` |
| Closed | `close_listener` | Closed | `Ok(())` |
| Closed | `accept` | Closed | `Closed` |
| Any | operation through another handler | Unchanged | `InvalidResource` |

### Stream transitions

The planned stream state machine has a read state, a write state, and a fully
closed state. A peer end and a local read shutdown are distinct read states.

| Current state | Operation and outcome | Next state | Observable result |
| --- | --- | --- | --- |
| Read open | `read` receives bytes | Unchanged | `ReadChunk` with a non-empty chunk |
| Read open | `read` receives peer end after buffered bytes are drained | Peer ended | `ReadEnd` |
| Peer ended | `read` | Unchanged | `ReadEnd` |
| Read open | `shutdown_read` | Read shut | `Ok(())`; unread buffered input is discarded and a blocked read returns `Closed` |
| Read shut | `read` | Unchanged | `Err(Closed)` |
| Read open | `read` times out or is cancelled | Read open | `Err(TimedOut)` or `Err(Cancelled)` |
| Read open | `read` has another failure | Read failed | `Err(error)` |
| Read failed | `read` | Unchanged | `Err(Closed)` |
| Write open | `write` commits bytes without failure | Unchanged | `Written(count)` |
| Write open | `write` times out or is cancelled | Write open | `WriteFailed(committed, error)` |
| Write open | `write` has another failure | Write failed | `WriteFailed(committed, error)` |
| Write failed | `write` | Unchanged | `WriteFailed(0, Closed)` |
| Write open | `shutdown_write` | Write closed | `Ok(())`; a blocked write returns `WriteFailed(committed, Closed)` |
| Write closed | `write` | Unchanged | `WriteFailed(0, Closed)` |
| Either half open | `close_stream` | Both closed | `Ok(())`; blocked operations return `Closed` |
| Both closed | `close_stream` | Both closed | `Ok(())` |
| Both closed | `read` | Both closed | `Err(Closed)` |
| Both closed | `write` | Both closed | `WriteFailed(0, Closed)` |
| Any | operation through another handler | Unchanged | A failure containing `InvalidResource` |

Under the proposed handler, a timeout or cancellation will not close a listener
or stream. A caller that cannot safely reuse a resource after an
application-level timeout must close it explicitly.

Dropping the last Veln reference will not define prompt cleanup under the
proposed handler. Applications will need to call the close functions. The
system handler must close all resources that it still owns when the handled
scope exits, including exits caused by a propagated error or runtime unwind.
Scope cleanup will be a safety net, not a substitute for explicit close when
peer-visible timing matters.

The proposed handler-owned safety net will be distinct from
[lexical deferred cleanup](../specification/execution.md#runtime-readiness-and-host-boundaries).
Application code will be able to register explicit close next to resource
acquisition without changing the handler's ownership boundary.

A resource must not escape its owning handled scope. Under the proposed
handler, a returned resource will already be closed by scope cleanup, and a
later operation under another handler will return `InvalidResource`.

## Compatibility and migration

The remaining implementation must put the system handler behind the effectful
public contract already owned by `std::net`, without creating a second public
network API.

1. Add private host intrinsics under a namespace that source imports cannot
   resolve.
2. Extend the exported `net.veln` with `net::system()` and `write_all`.
3. Update remaining standard-library code to import `net` and handle or propagate
   `net::IO` at its intended boundary.
4. Keep the existing compiler-known `net::...` spellings only as a temporary
   compatibility lowering when no imported module owns `net`.
5. Emit one migration diagnostic that points to `use net from "std"`, the
   `net::IO` effect, and the `net::system()` boundary.
6. Remove the compatibility lowering after all checked examples and standard
   package sources use the exported module.

An explicit `use net from "std"` always selects the standard module. The
compatibility lowering must not shadow that import.

## Acceptance model

Implementation is complete only when all rows have executable evidence. The
paths name intended evidence locations; they do not claim that the cases
already exist or pass.

| Concern | Input or event | Required observation | Intended evidence |
| --- | --- | --- | --- |
| System handling | A `net::IO` block is handled with `net::system()` | The remaining effects are host `net` and `time` | check cases and semantic tests |
| Host failure translation | A system operation receives an expected operating-system network failure | The operation returns a `NetError` with the applicable portable kind instead of producing a runtime diagnostic | table-driven host translation tests and loopback cases |
| Resolution | A name maps to repeated endpoints | Order is preserved and exact duplicates are removed | runtime conformance test |
| Listen and accept | The system handler listens on loopback port zero and a client connects | Reported listener address has an assigned port and accept returns a fresh stream | loopback run case |
| Connect failure | No server listens at a selected loopback address | `ConnectionRefused` or a documented portable fallback classification | loopback run case |
| Read end | Peer writes bytes and shuts down its write half | Bytes arrive before `ReadEnd`; later reads remain `ReadEnd` | loopback run case |
| Partial write | Scripted handler commits only a prefix | The write trace shows that `write_all` retries only the remaining suffix | runtime conformance test |
| Failed partial write | Scripted handler commits a prefix and reports a failure | `write_all` returns the reported `NetError` and performs no further write after `WriteFailed`, including when the outcome reports a non-zero committed count | runtime conformance test |
| Deadline | A scripted or loopback operation exceeds its deadline | `TimedOut`; the resource remains usable | run case |
| Cancellation | A token is cancelled while an operation is blocked | `Cancelled`; the resource remains usable | run case |
| Concurrent same-direction I/O | A second read or write starts before the first finishes | The second operation reports `Busy`; the first is unchanged | runtime conformance test |
| Close interruption | Listener or stream closes while an operation is blocked | Blocked operation returns `Closed` and does not hang | bounded loopback run case |
| Half-close | Local write half is shut down | Peer observes end; local reads can continue | loopback run case |
| Handler ownership | A resource from one handler is passed to another | `InvalidResource`; both handlers' states remain unchanged | runtime conformance test |
| Scope cleanup | A handled block exits with owned resources open | Host resources close and a peer observes closure | loopback run case |
| Escaped resource | A resource is returned from its owning handled scope | Scope cleanup closes it and another handler rejects it | runtime conformance test |
| Duplex adapter | A captured `Stream` is handled as `transport::DuplexStream` and protocol code reads and writes | The adapter uses `net::IO` and `write_all`, maps failures through the transport contract, and does not close or take lifecycle ownership of the stream | deterministic handler conformance test |
| Compatibility removal | Source uses a legacy compiler-known `net::...` spelling without `use net from "std"` after migration | The compatibility lowering no longer resolves the spelling, and the migration diagnostic identifies the current import, effect, and handler boundary | check and diagnostic cases |
| Package docs | Standard package documentation is generated after `net::system` and `write_all` are implemented | The newly implemented handler, helper, and their effectful examples appear alongside the existing `net` declarations | package-documentation gate |

Loopback cases must bind only loopback addresses and must use bounded deadlines.
They must not require external DNS or internet access. Cases that validate
resolver ordering will use the planned deterministic handler.

## Specification promotion

The public contract, direct facade, and their executable evidence are current
behavior. For the remaining work, add executable evidence before describing
the implementation as current behavior. Extend the smallest focused current
specification pages for:

- the `net::system()` implementation of the current `net::IO` boundary;
- listener and stream lifecycle transitions;
- the `transport::DuplexStream` adapter boundary.

Update package documentation and language-service standard-library symbol
evidence when a newly implemented remaining API, such as `system` or
`write_all`, requires that evidence. Remove this proposal and its catalog entry
only after the current specification and executable cases cover every
acceptance row.
