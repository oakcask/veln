---
role: proposal
update-when: The planned standard-library network API, network effect boundary, host handler, stream-resource lifecycle, or lexical-cleanup prerequisite changes.
---

# Standard-library networking and its effect boundary

## Outcome

The standard package will export a `net` module for portable stream-oriented
networking. Library code will use `net::listen`, `net::accept`, `net::connect`,
name resolution, stream I/O, and resource cleanup through one public algebraic
effect. An application will select the host implementation by handling that
effect with `net::system()`.

This keeps two concerns separate:

- `net::IO` is the substitutable contract used by application and protocol
  code.
- The existing host `net` and `time` effects remain the trusted runtime
  boundary used by the system handler.

The first delivery covers TCP stream clients and servers, address text,
resolution, deadlines, cancellation, and cleanup. It does not attempt full API
parity with another language's network library.

## Motivation

Veln already has host-backed socket operations and opaque listener and stream
values. Those operations are compiler-known symbols instead of an exported
standard-package module. Code can call them, but the public API shape, ordinary
failure values, resource state, and substitution boundary are not owned by a
standard-library source module.

Go's `net` package supplies a useful division of responsibility: dialing,
listening, accepting, address inspection, name resolution, and connection I/O
belong to one portable package, while protocol libraries build above its
connection abstraction. Veln should use that division without copying Go's
method model or mutable deadline API. The source references are the
[Go `net` package overview](https://pkg.go.dev/net) and the
[Go connection contracts](https://go.dev/src/net/net.go).

Veln also needs a visible answer to a question that Go does not have: which
effect represents network access, and where can a program replace its
implementation? A public effect plus a standard host handler gives library
authors one effect to declare and gives the runtime test harness a
deterministic interception point.

## Goals

- Export `net.veln` from the `std` package.
- Give stream client and server code one portable API for TCP, TCP over IPv4,
  and TCP over IPv6.
- Return typed ordinary failures instead of turning expected operating-system
  outcomes into runtime diagnostics.
- Make DNS resolution and socket resource operations replaceable by an effect
  handler.
- Preserve precise end-of-stream, deadline, cancellation, partial-write, and
  close outcomes.
- Keep protocol code independent of host socket handles through
  `transport::DuplexStream` when it needs only duplex byte transport.
- Provide executable acceptance evidence for the system handler and for a
  deterministic test handler.

## Non-goals

The first delivery does not include:

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

## Public module

A package imports the module explicitly:

```veln
use net from "std"
```

The `net` module exports the following values. Names in this section are the
proposed source contract rather than aliases for compiler-known public
symbols.

### Network and address values

```veln
pub type Network
	Tcp
	Tcp4
	Tcp6
end

pub type Address
	Address(network: Network, host: String, port: Int)
end

pub type Endpoint
	Endpoint(network: Network, address: String)
end
```

`Address` is unresolved input. `host` may be a DNS name, an IP literal, or the
empty string for a wildcard listening address. `port` must be in the inclusive
range from zero through 65535. Port zero requests an ephemeral local port when
listening. Connecting to port zero is permitted to reach platforms that assign
meaning to it; any rejection is a `NetError`.

`Endpoint` is a resolved numeric address returned by the handler. Its
`address` uses host-port text. An IPv6 host is bracketed. The module supplies
pure helpers:

```veln
pub fn join_host_port(host: String, port: Int) -> Result<String, NetError>
pub fn split_host_port(address: String) -> Result<{ host : String, port : Int }, NetError>
```

`join_host_port` rejects an out-of-range port. `split_host_port` requires one
port, accepts bracketed IPv6, and rejects ambiguous unbracketed IPv6. Neither
helper performs name resolution or requires an effect.

### Resources and counts

```veln
pub type Listener = NetListener
pub type Stream = NetStream
pub type ByteCount = prelude::ByteCount
```

`Listener` and `Stream` are public names for runtime-backed opaque resource
references. Their representation is not source-constructible, serializable,
or a cross-process identity. A reference belongs to the `net::IO` handler that
created it. Passing it to a different handler returns `InvalidResource`.

`ByteCount` is non-negative. The existing byte-chunk conversion helpers are
used to compare a count with a `ByteChunk` length.

### Errors and read outcomes

```veln
pub type NetErrorKind
	InvalidAddress
	UnsupportedNetwork
	NameNotFound
	PermissionDenied
	AddressInUse
	ConnectionRefused
	ConnectionReset
	TimedOut
	Cancelled
	Closed
	Busy
	InvalidResource
	Other
end

pub type NetError
	NetError(operation: String, network: Option<Network>, address: Option<String>, kind: NetErrorKind, message: String)
end

pub type ReadOutcome
	ReadChunk(bytes: ByteChunk)
	ReadEnd
end

pub type WriteOutcome
	Written(count: ByteCount)
	WriteFailed(committed: ByteCount, error: NetError)
end
```

`operation` is the stable `net` operation name, such as `listen`, `accept`, or
`read`. `kind` is the portable classification. `message` is explanatory host
text and is not a stable comparison key. The system handler must not expose a
platform error number as the only classification.

Expected host failures use `NetError`. A runtime diagnostic is reserved for a
broken runtime invariant, such as internal resource-table corruption.

`WriteOutcome` preserves a committed prefix when the host reports bytes and a
failure from the same write. `committed` is in the inclusive range from zero
through the input length. A caller must not retry that prefix.

For an operation whose declared result is `Result`, `Busy` and
`InvalidResource` are returned through `Err`. For `write`, those failures are
`WriteFailed(0, error)`.

### The `net::IO` effect

```veln
pub effect IO
	resolve(address: Address) -> Result<List<Endpoint>, NetError>
	listen(address: Address) -> Result<Listener, NetError>
	connect(address: Address, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<Stream, NetError>
	accept(listener: Listener, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<Stream, NetError>
	listener_address(listener: Listener) -> Result<Endpoint, NetError>
	close_listener(listener: Listener) -> Result<(), NetError>
	read(stream: Stream, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<ReadOutcome, NetError>
	write(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>) -> WriteOutcome
	local_address(stream: Stream) -> Result<Endpoint, NetError>
	peer_address(stream: Stream) -> Result<Endpoint, NetError>
	shutdown_read(stream: Stream) -> Result<(), NetError>
	shutdown_write(stream: Stream) -> Result<(), NetError>
	close_stream(stream: Stream) -> Result<(), NetError>
end
```

The effect operations are the complete handler contract. Application code uses
the following functions:

```veln
pub fn resolve(address: Address) -> Result<List<Endpoint>, NetError> effects [net::IO]
pub fn listen(address: Address) -> Result<Listener, NetError> effects [net::IO]
pub fn connect(address: Address) -> Result<Stream, NetError> effects [net::IO]
pub fn connect_with(address: Address, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<Stream, NetError> effects [net::IO]
pub fn accept(listener: Listener) -> Result<Stream, NetError> effects [net::IO]
pub fn accept_with(listener: Listener, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<Stream, NetError> effects [net::IO]
pub fn listener_address(listener: Listener) -> Result<Endpoint, NetError> effects [net::IO]
pub fn close_listener(listener: Listener) -> Result<(), NetError> effects [net::IO]
pub fn read(stream: Stream) -> Result<ReadOutcome, NetError> effects [net::IO]
pub fn read_with(stream: Stream, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<ReadOutcome, NetError> effects [net::IO]
pub fn write(stream: Stream, bytes: ByteChunk) -> WriteOutcome effects [net::IO]
pub fn write_with(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>) -> WriteOutcome effects [net::IO]
pub fn local_address(stream: Stream) -> Result<Endpoint, NetError> effects [net::IO]
pub fn peer_address(stream: Stream) -> Result<Endpoint, NetError> effects [net::IO]
pub fn shutdown_read(stream: Stream) -> Result<(), NetError> effects [net::IO]
pub fn shutdown_write(stream: Stream) -> Result<(), NetError> effects [net::IO]
pub fn close_stream(stream: Stream) -> Result<(), NetError> effects [net::IO]
pub fn write_all(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<(), NetError> effects [net::IO]
```

The short `connect`, `accept`, `read`, and `write` functions perform their
effect operation with absent deadline and cancellation values. Their `*_with`
counterparts pass both options unchanged. Direct `perform` expressions remain
valid for handler and effect-polymorphism tests.

An absent deadline means that elapsed time does not end the operation. An
absent cancellation token means that cancellation does not end the operation.
If both are present and observable before success, the first condition observed
by the handler determines `TimedOut` or `Cancelled`. An operation may still
return another host failure that was already committed before either condition
was observed.

`resolve` preserves the handler's preferred endpoint order and removes exact
duplicates. It returns `NameNotFound` when no endpoint is available. `connect`
may try resolved endpoints, but it returns only one stream or one error.

`read` returns `ReadEnd` only after the peer's write half has ended and all
previously received bytes have been returned. It never uses an empty chunk as
an end marker.

`write` returns the count committed to the stream. `Written` may contain a
count smaller than the input length. The caller may retry only the uncommitted
suffix. `WriteFailed` reports both the committed prefix and the failure. The
caller processes the committed count before handling the error and does not
retry any committed bytes.

`write_all` repeats the write operation for the uncommitted suffix until all
bytes are written or an error occurs. It stops after `WriteFailed`, including
when that outcome committed a non-empty prefix. A zero-byte `Written` outcome
before completion becomes `NetError` with kind `Other`; this prevents an
unbounded retry loop.

## Handler model

### System handler

The module exports:

```veln
pub handler system() handles net::IO effects [net, time]
```

The handler translates portable operations to private host intrinsics. The
private intrinsics are compiler/runtime implementation details and are not
source-resolvable package APIs. Handling `net::IO` therefore removes that
effect and introduces only the host `net` and `time` effects.

The handler has one fixed effect set, so an untimed operation also introduces
host `time`. This first contract accepts that conservative effect because
splitting timed and untimed operations across handlers would split ownership
of the same resources. A later change may remove the extra host effect only if
handler effect inference can do so without changing the `net::IO` API.

Reusable code declares `net::IO`:

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

The application chooses the host boundary:

```veln
pub fn main() -> Result<(), net::NetError> effects [net, time]
	handle serve_once(net::Address(net::Tcp, "127.0.0.1", 8080)) with net::system()
end
```

No system handler is inserted implicitly for an arbitrary function. The entry
point must apply it, or a caller must propagate `net::IO`. This keeps network
authority visible during composition and makes missing-handler behavior a
static effect error.

### Test handlers

The runtime conformance harness supplies a deterministic `net::IO` handler.
Like the system handler, it is trusted to create opaque `Listener` and `Stream`
references. It is test infrastructure, not an exported standard-package
module. A source-defined handler may deny, trace, or delegate operations, but
it cannot fabricate a successful resource reference through a public
constructor.

A deterministic handler must be able to script:

- resolution results and failures;
- incoming connections and accepted stream identities;
- read chunks, end-of-stream, and read failures;
- partial writes and write failures;
- deadline and cancellation outcomes;
- the local and peer endpoints of each resource.

The handler records operations in call order. The host-side test harness
inspects that record to verify cleanup and retry behavior. Its script and trace
formats are repository-internal test data and are not public Veln APIs.

### Duplex transport adapter

`transport::DuplexStream` remains the narrow effect for a protocol that already
owns one connected stream. The standard library provides an adapter handler
that captures a `net::Stream`, performs `net::read` and `net::write_all`, and
maps `NetError` into the transport failure type selected by the transport
contract.

The adapter does not listen, accept, resolve, connect, or close the captured
stream. The caller retains lifecycle ownership. This prevents a protocol
handler from silently closing a stream that another layer intends to reuse.

## Resource lifecycle

The handler owns resource state. Operations on different resources may proceed
concurrently. For one stream, at most one read and one write may be in flight;
one read and one write may proceed concurrently. A second concurrent read or a
second concurrent write returns `Busy` until the active operation finishes.
This avoids an unspecified byte split between callers.

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

A stream has a read state, a write state, and a fully closed state. A peer end
and a local read shutdown are distinct read states.

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

A timeout or cancellation does not close a listener or stream. A caller that
cannot safely reuse a resource after an application-level timeout must close it
explicitly.

Dropping the last Veln reference does not define prompt cleanup. Applications
must call the close functions. The system handler closes all resources that it
still owns when the handled scope exits, including exits caused by a propagated
error or runtime unwind. Scope cleanup is a safety net, not a substitute for
explicit close when peer-visible timing matters.

The handler-owned safety net is distinct from
[lexical deferred cleanup](../specification/execution.md#runtime-readiness-and-host-boundaries).
Application code can register explicit close next to resource acquisition
without changing the handler's ownership boundary.

A resource must not escape its owning handled scope. A returned resource is
already closed by scope cleanup, and a later operation under another handler
returns `InvalidResource`.

## Compatibility and migration

Implementation must move the public contract into `std::net` without leaving
two independently maintained network APIs.

1. Add private host intrinsics under a namespace that source imports cannot
   resolve.
2. Implement and export `net.veln`, its `net::IO` wrappers, and `net::system()`.
3. Update standard-library code to import `net` and handle or propagate
   `net::IO` at its intended boundary.
4. Keep the existing compiler-known `net::...` spellings only as a temporary
   compatibility lowering when no imported module owns `net`.
5. Emit one migration diagnostic that points to `use net from "std"`, the
   `net::IO` effect, and the `net::system()` boundary.
6. Remove the compatibility lowering after all checked examples and standard
   package sources use the exported module.

An explicit `use net from "std"` always selects the standard module. The
compatibility lowering must not shadow that import. Package documentation,
definition, references, completion, hover, and rename must resolve public
network symbols to `std::net`, not to synthetic compiler descriptors.

## Acceptance model

Implementation is complete only when all rows have executable evidence. The
paths name intended evidence locations; they do not claim that the cases
already exist or pass.

| Concern | Input or event | Required observation | Intended evidence |
| --- | --- | --- | --- |
| Export | `use net from "std"` | `net::listen` and every declared public type resolve to `std::net` | package and check cases under `examples/specification/` |
| Effects | A public function calls `net::listen` without `net::IO` | Static diagnostic names `net::IO` and the call site | check cases |
| System handling | A `net::IO` block is handled with `net::system()` | The remaining effects are host `net` and `time` | check cases and semantic tests |
| No implicit authority | An entry point leaves `net::IO` unhandled | Static failure; the runner does not install a handler | check and run cases |
| Address parsing | Bracketed IPv6 and a valid port are split and rejoined | Stable host and port values | standard-library doctests |
| Address rejection | Port is out of range or IPv6 is ambiguous | `InvalidAddress`; no network operation is recorded | doctests and runtime conformance test |
| Resolution | A name maps to repeated endpoints | Order is preserved and exact duplicates are removed | runtime conformance test |
| Listen and accept | The system handler listens on loopback port zero and a client connects | Reported listener address has an assigned port and accept returns a fresh stream | loopback run case |
| Connect failure | No server listens at a selected loopback address | `ConnectionRefused` or a documented portable fallback classification | loopback run case |
| Read end | Peer writes bytes and shuts down its write half | Bytes arrive before `ReadEnd`; later reads remain `ReadEnd` | loopback run case |
| Partial write | Scripted handler commits only a prefix | `write_all` retries only the remaining suffix | runtime conformance test |
| Failed partial write | Scripted handler commits a prefix and reports a failure | `WriteFailed` preserves the count and `write_all` does not retry it | runtime conformance test |
| Deadline | A scripted or loopback operation exceeds its deadline | `TimedOut`; the resource remains usable | run case |
| Cancellation | A token is cancelled while an operation is blocked | `Cancelled`; the resource remains usable | run case |
| Concurrent same-direction I/O | A second read or write starts before the first finishes | The second operation reports `Busy`; the first is unchanged | runtime conformance test |
| Close interruption | Listener or stream closes while an operation is blocked | Blocked operation returns `Closed` and does not hang | bounded loopback run case |
| Half-close | Local write half is shut down | Peer observes end; local reads can continue | loopback run case |
| Handler ownership | A resource from one handler is passed to another | `InvalidResource`; both handlers' states remain unchanged | runtime conformance test |
| Scope cleanup | A handled block exits with owned resources open | Host resources close and a peer observes closure | loopback run case |
| Escaped resource | A resource is returned from its owning handled scope | Scope cleanup closes it and another handler rejects it | runtime conformance test |
| Editor identity | Definition or hover targets an imported network symbol | Location and package identity are `std::net` | LSP and MCP cases |
| Package docs | Standard package documentation is generated | `net` API, effects, errors, and examples are present | package-documentation gate |

Loopback cases must bind only loopback addresses and must use bounded deadlines.
They must not require external DNS or internet access. Cases that validate
resolver ordering use the deterministic handler.

## Specification promotion

When implementation begins, add executable evidence before describing the API
as current behavior. Then add the smallest focused current specification pages
for:

- the exported `std::net` API and error contract;
- the `net::IO` and `net::system()` effect boundary;
- listener and stream lifecycle transitions;
- the `transport::DuplexStream` adapter boundary.

Update package documentation and the language-service standard-library symbol
evidence in the same change. Remove this proposal and its catalog entry only
after the current specification and executable cases cover every acceptance
row.
