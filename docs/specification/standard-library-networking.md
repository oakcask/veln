---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#network-operation-boundary; limits=#limits-and-errors
update-when: The exported std net values, functions, system handler, resource lifecycle, host-port behavior, effect boundary, or executable network examples change.
---

# Standard-library Networking

The exported `std::net` module provides portable network values, pure host-port
text helpers, and the substitutable `net::IO` contract used by stream-network
code. Applications and tests select a handler explicitly; the module does not
install network authority.

## Usage

Import the module explicitly:

```veln
use net from "std"
```

The module exports these public values and aliases:

| Value | Public shape |
| --- | --- |
| `Network` | `Tcp`, `Tcp4`, and `Tcp6` variants |
| `Address` | `Address(network: Network, host: String, port: Int)` |
| `Endpoint` | `Endpoint(network: Network, address: String)` |
| `NetErrorKind` | `InvalidAddress`, `UnsupportedNetwork`, `NameNotFound`, `PermissionDenied`, `AddressInUse`, `ConnectionRefused`, `ConnectionReset`, `TimedOut`, `Cancelled`, `Closed`, `Busy`, `InvalidResource`, and `Other` variants |
| `NetError` | `NetError(operation: String, network: Option<Network>, address: Option<String>, kind: NetErrorKind, message: String)` |
| `Listener` | Alias for the opaque `NetListener` resource |
| `Stream` | Alias for the opaque `NetStream` resource |
| `ByteCount` | Alias for `prelude::ByteCount` |
| `ReadOutcome` | `ReadChunk(bytes: ByteChunk)` and `ReadEnd` variants |
| `WriteOutcome` | `Written(count: ByteCount)` and `WriteFailed(committed: ByteCount, error: NetError)` variants |

The algebraic value constructors and fields can be matched as ordinary public
data. `Listener` and `Stream` are the opaque aliases described under
[Limits And Errors](#limits-and-errors), so they do not expose constructors.
Constructing an `Address` or `Endpoint` does not resolve a name, open a socket,
or validate the contained port or address text.

Code compares a `NetError` kind rather than its explanatory message. The pure
host-port helpers produce `InvalidAddress`. An `IO` handler can return the
other kinds as ordinary failures.

An application selects the host implementation explicitly:

```veln
fn connect_and_close() -> Result<(), net::NetError> effects [net::IO]
	let stream = net::connect(net::Address(net::Tcp, "example.test", 443))?
	net::close_stream(stream)
end

pub fn main() -> Result<(), net::NetError> effects [net, time]
	handle connect_and_close() with net::system()
end
```

`net::system()` is a public handler for `net::IO` with retained effects
`[net, time]`. Handling removes `net::IO` and exposes the host network and
clock boundaries. The fixed effect set also applies when one operation does
not inspect a deadline.

## Address Values And Host-port Text

The pure helper signatures are:

```veln
pub fn join_host_port(host: String, port: Int) -> Result<String, NetError>
pub fn split_host_port(address: String) -> Result<{ host : String, port : Int }, NetError>
```

`join_host_port(host, port)` returns `host:port` for a hostname, IPv4 literal,
or empty wildcard host. If the host contains a colon, the result encloses it in
square brackets, so an IPv6 literal such as `2001:db8::1` becomes
`[2001:db8::1]:443`. The helper accepts ports from zero through 65535,
inclusive.

`split_host_port(address)` returns a structural record with `host` and `port`
fields. It accepts the unbracketed hostname and IPv4 form and the bracketed
IPv6 form produced by `join_host_port`. The returned host excludes brackets.
The operation is text processing only: it does not perform DNS lookup, inspect
network interfaces, or produce a host network effect.

The executable network address example under `examples/specification/run/`
checks value construction, representative round trips, port boundaries, and
the rejection rules.

## Network Operation Boundary

`net::IO` declares the complete source-level handler contract:

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

The module exports one direct forwarding function for each operation. It also
exports `connect_with`, `accept_with`, `read_with`, and `write_with` for the
operations that accept deadline and cancellation options. Each function
performs exactly one matching `net::IO` operation and returns its result
unchanged. The short `connect`, `accept`, `read`, and `write` functions supply
`None` for both options. Each `*_with` function passes both supplied options
unchanged.

The complete-write helper has this signature:

```veln
pub fn write_all(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<(), NetError> effects [IO]
```

`write_all(stream, bytes, deadline, token)` writes the complete `ByteChunk`.
It returns `Ok(())` without performing `IO::write` when the input is empty.
After `Written(count)` commits a proper prefix, it performs another write with
only the uncommitted suffix. Every attempt receives the original stream,
deadline, and cancellation token.

`write_all` returns the reported `NetError` immediately after `WriteFailed`,
including an outcome with a non-zero committed count. It performs no later
write after that failure. If `Written(0)` occurs while input remains,
`write_all` returns a `NetError` whose operation is `write_all` and whose kind
is `Other`; this prevents an unbounded retry. A `Written` count larger than the
attempted chunk is also an `Other` failure because it violates the handler
contract.

`ReadOutcome` has the `ReadChunk(ByteChunk)` and `ReadEnd` shapes. The direct
`read` and `read_with` functions return the handler's outcome unchanged,
including `ReadChunk` with an empty `ByteChunk`. The facade does not enforce
the system handler's non-empty read-chunk rule.

A public caller declares `effects [net::IO]`, handles that effect explicitly,
or receives a static missing-effect diagnostic. A runnable entry that retains
`net::IO` is rejected; the runner does not install a handler implicitly. A
source-defined scoped handler can return ordinary failures and can observe
arguments that do not require constructing an opaque resource.

The system handler resolves and opens TCP, TCP4, and TCP6 endpoints. Resolution
preserves the host resolver's preferred order and removes exact duplicate
endpoints. An empty result is `NameNotFound`. The deadline and cancellation
token supplied to `connect_with` bound both name resolution and every endpoint
attempt. An already expired deadline or cancelled token prevents resolution
from starting. A waiting `resolve`, hostname `listen`, or `connect_with` caller
observes task cancellation during platform resolution.
All platform name resolution used by `resolve`, hostname `listen`, and
`connect_with` shares finite worker capacity and retains no queued work. If
every resolver worker is occupied, the operation returns `Busy` without
creating another worker. A platform resolver call that ignores interruption
can keep its worker occupied after the waiting caller returns, but it cannot
create unbounded resolver work. Scoped Veln resolver handlers remain on the
calling execution and do not use the platform worker pool.

Listen on port zero reports the assigned local port. Expected address,
resolver, socket, deadline, cancellation, and lifecycle failures are returned
as `NetError`; an unrecognized failure reported through the typed host boundary
uses `Other`. A Veln runtime failure, an unexpected unchecked host failure, or
a JVM error remains an abrupt failure. It is not converted to `Other`, does not
trigger another endpoint attempt, and still unwinds handler cleanup.

### Host Failure Translation

The system handler translates a typed host failure category as follows:

| Host category | `NetErrorKind` |
| --- | --- |
| `timed_out` | `TimedOut` |
| `cancelled` | `Cancelled` |
| `closed` | `Closed` |
| `connection_refused` | `ConnectionRefused` |
| `connection_reset` | `ConnectionReset` |
| `address_in_use` | `AddressInUse` |
| `permission_denied` | `PermissionDenied` |
| `name_not_found` | `NameNotFound` |
| `invalid_endpoint` or `invalid_input` | `InvalidAddress` |
| Any other category | `Other` |

The direct JVM host boundary classifies the corresponding platform failures
with the same specific kinds. Resolver-capacity exhaustion uses `Busy`.
Resource ownership failures use `InvalidResource`. A bind failure is
`AddressInUse` only when the requested local endpoint belongs to the host; a
bind to an unavailable local address uses the `Other` fallback rather than
claiming that a port collision occurred.

System reads return non-empty `ReadChunk` values. They return `ReadEnd` after
the peer write half ends and buffered bytes have been consumed; later reads
remain `ReadEnd`. A write reports the bytes committed by that attempt, including
a committed prefix on failure. An empty write returns `Written(ByteCount(0))`
after resource ownership, open-state, and same-direction concurrency checks
succeed; it does not inspect the supplied deadline or cancellation token. For
a non-empty stream backed directly by a host socket, `write` waits through
socket backpressure until it commits the complete chunk or an interruption or
failure occurs. Success reports the input byte count. `WriteFailed` reports the
exact committed prefix, including a non-zero prefix when close, shutdown,
deadline expiration, cancellation, or another transport failure interrupts the
write. A deadline or cancellation failure leaves an otherwise reusable
listener or stream open.

Connection establishment is the commit point. During resolution and endpoint
attempts, each control check tests cancellation before deadline expiration.
An endpoint attempt that reports cancellation or deadline expiration ends the
connection operation; the handler does not try a later resolved endpoint.
Once the host reports successful establishment, the handler returns the
connected stream without another deadline or cancellation check. A control
that becomes observable only at or after that commit does not replace success.

Each listener permits one active accept; a second accept returns `Busy`. Each
stream permits one active read and one active write at the same time. A second
operation in the same direction returns `Busy`. Closing a listener or stream
interrupts its blocked operations with `Closed`. Read and write shutdown affect
only the selected half, and close operations are idempotent.

Listener state transitions are:

| Current state | Event | Next state and result |
| --- | --- | --- |
| Open | `accept` succeeds | The listener stays open and returns a fresh open stream. |
| Open | `accept` times out or is cancelled | The listener stays open and returns `TimedOut` or `Cancelled`. |
| Open | The host reports clean accept end | The listener stays open and returns `Closed`. |
| Open | `close_listener` | The listener becomes closed, returns `Ok(())`, and interrupts blocked accepts with `Closed`. |
| Open | `close_listener` fails before host closure commits | The listener stays open, returns the error, retains its cleanup obligation, and a later explicit close retries the host operation. |
| Open | `close_listener` reports failure after host closure commits | The listener becomes confirmed closed and returns the error; a later close is idempotently successful. |
| Open | `close_listener` fails with unknown commit state | The listener becomes close-uncertain, returns the error, rejects ordinary operations with `Closed`, retains its cleanup obligation, and a later explicit close retries. |
| Closed | `close_listener` | The listener stays closed and returns `Ok(())`. |
| Closed | `accept` | The listener stays closed and returns `Closed`. |
| Any | Another handler uses the listener | The listener does not change and the operation returns `InvalidResource`. |

A stream tracks its read and write halves independently:

| Current state | Event | Next state and result |
| --- | --- | --- |
| Read open | `read` receives bytes | The read half stays open and returns a non-empty `ReadChunk`. |
| Read open | The peer ends its write half after buffered bytes drain | The read half becomes peer-ended and returns `ReadEnd`; later reads also return `ReadEnd`. |
| Read open | `shutdown_read` | The read half becomes shut, unread buffered input is discarded, and a blocked or later read returns `Closed`. |
| Read open | `shutdown_read` fails without committing | The read half stays open and retryable. The call returns the error, and a concurrent successful read keeps its bytes. |
| Read open | `shutdown_read` reports a committed failure | The read half becomes shut and the call returns the error; a concurrent blocked read observes `Closed`. |
| Read open | `shutdown_read` fails with unknown commit state | The read half becomes shutdown-uncertain and unavailable to reads until an explicit shutdown retry confirms its state. |
| Read open | `read` times out or is cancelled | The read half stays open and returns `TimedOut` or `Cancelled`. |
| Read open | `read` has another failure | The read half becomes failed and returns that error; later reads return `Closed`. |
| Write open | `write` commits without failure | The write half stays open and returns the committed count. |
| Write open | `write` times out or is cancelled | The write half stays open and returns the committed prefix with `TimedOut` or `Cancelled`. |
| Write open | `write` has another failure | The write half becomes failed and returns the committed prefix with that error; later writes return zero committed with `Closed`. |
| Write open | `shutdown_write` | The write half becomes shut, returns `Ok(())`, and a blocked or later write returns its committed prefix with `Closed`. |
| Write open | `shutdown_write` fails without committing | The write half stays open and retryable. The call returns the error, and a concurrent successful write keeps its exact committed prefix. |
| Write open | `shutdown_write` reports a committed failure | The write half becomes shut and the call returns the error; a concurrent blocked write reports its committed prefix with `Closed`. |
| Write open | `shutdown_write` fails with unknown commit state | The write half becomes shutdown-uncertain and unavailable to writes until an explicit shutdown retry confirms its state. |
| Either half open | `close_stream` | Both halves become closed, the call returns `Ok(())`, and blocked operations return `Closed`. |
| Not closed | `close_stream` fails before host closure commits | Both halves keep their prior state, the call returns the error, the cleanup obligation remains, and a later explicit close retries the host operation. |
| Not closed | `close_stream` reports failure after host closure commits | Both halves become confirmed closed and the call returns the error; a later close is idempotently successful. |
| Not closed | `close_stream` fails with unknown commit state | The stream becomes close-uncertain, the call returns the error, ordinary operations return `Closed`, the cleanup obligation remains, and a later explicit close retries. |
| Both halves closed | `close_stream` | Both halves stay closed and the call returns `Ok(())`. |
| Any | Another handler uses the stream | The stream does not change and the operation returns `InvalidResource`. |

Close and shutdown uncertainty is monotonic. After an operation reports an
unknown commit state, a retry that fails before committing does not restore the
resource or half to its earlier open state. Ordinary operations continue to
return `Closed`, the cleanup obligation remains, and another explicit retry is
allowed. A successful retry or a failure that confirms the close or shutdown
commit moves the affected state to confirmed closed or shut. Later close or
shutdown calls then succeed idempotently.

The system handler instance owns every listener and stream it creates. A
different handler returns `InvalidResource` without changing either handler's
state. Leaving the handled scope closes every resource still owned by that
instance, including normal return, propagated failure, and runtime unwind. A
resource returned from the scope is therefore closed and another handler
rejects it as `InvalidResource`.

Scope exit also closes the owner to new resources before cleanup starts. If an
inherited child operation finishes `listen`, `connect`, or `accept` after that
point, the operation does not return the new resource. It applies the same
committed, uncommitted, or unknown close outcome rules to that resource and
returns `InvalidResource`. Scope exit waits for already admitted resource
producers and retries retained cleanup obligations to a fixed bound. If that
bound cannot confirm closure, scope exit fails instead of reporting normal
completion. Cleanup still attempts the owner's other resources before that
failure. The resource never transfers to the restored outer handler.

## Limits And Errors

Both helpers return `Err(NetError(... InvalidAddress ...))` when a port is less
than zero or greater than 65535. `split_host_port` also returns
`InvalidAddress` when the address has no port, has an empty or non-decimal
port, contains more than one unbracketed colon, or has a bracketed host without
the required closing-bracket-and-port separator. In particular, an IPv6 host
must be bracketed because its unbracketed colons do not identify one port
unambiguously.

The helpers do not validate DNS spelling or the internal syntax of an IP
literal. `Listener` and `Stream` are opaque; source code cannot construct a
successful resource reference for a fake handler.

The system handler supports TCP streams only. It does not provide UDP,
Unix-domain sockets, TLS, HTTP, proxies, packet APIs, file-descriptor conversion,
or platform-specific socket options. Prompt peer-visible cleanup still requires
an explicit close; scope cleanup is a safety net. Adapting an owned stream to
`transport::DuplexStream`, publishing public handler declarations in package
documentation, and removing the legacy compiler-known network compatibility
surface remain proposal work.

## References

The exported implementation and companion tests are in
`crates/veln-stdlib/veln/net.veln` and
`crates/veln-stdlib/veln/net.test.veln`. The companion tests cover the values
that source code can construct, `write_all`, scoped resolution and failure
translation, exact write progress, concurrency, interruption decisions, and
lifecycle uncertainty across retries.
The JVM backend effect-injection tests use separate Veln test support to
exercise the facade with opaque listener and stream resources and to check
resource, option, byte, and result preservation. Checked command-level examples
under `examples/specification/check/` and `examples/specification/run/` cover
the explicit standard-module identity, nominal effect requirement, and
unhandled runner boundary. The bounded
`standard-library-network-system-handler` loopback case checks real JVM host
integration for port-zero listening, addresses, byte transfer, peer end,
deadline and cancellation reuse, half-close, ownership rejection, and
peer-observed listener and stream cleanup after normal return, propagated
failure, and runtime unwind. Same-direction `Busy` observations establish that
the interrupted accept, read, and backpressured write operations are in flight.
The JVM lifecycle harness also checks that resource creation racing with scope
cleanup cannot publish a listener or stream after its owner closes and that an
uncommitted retry does not clear earlier lifecycle uncertainty. The focused
effect-injection case checks that a cleanup failure during propagated-result
unwind still runs enclosing deferred cleanup and restores enclosing handlers.
