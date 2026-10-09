---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#network-operation-boundary; limits=#limits-and-errors
update-when: The exported std net value types or functions, host-port helper behavior, network effect boundary, or executable network examples change.
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
pub fn main() -> Result<(), net::NetError> effects [net, time]
	handle net::connect(net::Address(net::Tcp, "example.test", 443)) with net::system()
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
the proposed system handler's non-empty read-chunk rule.

A public caller declares `effects [net::IO]`, handles that effect explicitly,
or receives a static missing-effect diagnostic. A runnable entry that retains
`net::IO` is rejected; the runner does not install a handler implicitly. A
source-defined scoped handler can return ordinary failures and can observe
arguments that do not require constructing an opaque resource.

The system handler resolves and opens TCP, TCP4, and TCP6 endpoints. Resolution
preserves the host resolver's preferred order and removes exact duplicate
endpoints. An empty result is `NameNotFound`. Listen on port zero reports the
assigned local port. Expected address, resolver, socket, deadline, cancellation,
and lifecycle failures are returned as `NetError`; an unrecognized host failure
uses `Other` instead of becoming a runtime diagnostic.

System reads return non-empty `ReadChunk` values. They return `ReadEnd` after
the peer write half ends and buffered bytes have been consumed; later reads
remain `ReadEnd`. A write reports the bytes committed by that attempt, including
a committed prefix on failure. A deadline or cancellation failure leaves an
otherwise reusable listener or stream open.

Each stream permits one active read and one active write at the same time. A
second operation in the same direction returns `Busy`. Closing a listener or
stream interrupts its blocked operations with `Closed`. Read and write shutdown
affect only the selected half, and close operations are idempotent.

The system handler instance owns every listener and stream it creates. A
different handler returns `InvalidResource` without changing either handler's
state. Leaving the handled scope closes every resource still owned by that
instance, including normal return, propagated failure, and runtime unwind. A
resource returned from the scope is therefore closed and another handler
rejects it as `InvalidResource`.

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
`transport::DuplexStream` and removing the legacy compiler-known network
compatibility surface remain proposal work.

## References

The exported implementation and companion tests are in
`crates/veln-stdlib/veln/net.veln` and
`crates/veln-stdlib/veln/net.test.veln`. The companion tests cover the values
that source code can construct and the `write_all` retry and failure decisions.
The JVM backend effect-injection tests use separate Veln test support to
exercise the facade with opaque listener and stream resources and to check
resource, option, byte, and result preservation. Checked command-level examples
under `examples/specification/check/` and `examples/specification/run/` cover
the explicit standard-module identity, nominal effect requirement, and
unhandled runner boundary. The bounded
`standard-library-network-system-handler` loopback case covers the system
handler's typed outcomes, resource transitions, ownership, and cleanup.
