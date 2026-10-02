---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#address-values-and-host-port-text; limits=#limits-and-errors
update-when: The exported std net value types, host-port helper behavior, network effect boundary, or executable network examples change.
---

# Standard-library Networking

The exported `std::net` module provides portable network names, unresolved and
resolved address values, network error values, and pure host-port text helpers.
It does not yet provide DNS, sockets, network effects, or a system handler.

## Usage

Import the module explicitly:

```veln
use net from "std"
```

The module exports these algebraic values:

| Value | Public shape |
| --- | --- |
| `Network` | `Tcp`, `Tcp4`, and `Tcp6` variants |
| `Address` | `Address(network: Network, host: String, port: Int)` |
| `Endpoint` | `Endpoint(network: Network, address: String)` |
| `NetErrorKind` | `InvalidAddress`, `UnsupportedNetwork`, `NameNotFound`, `PermissionDenied`, `AddressInUse`, `ConnectionRefused`, `ConnectionReset`, `TimedOut`, `Cancelled`, `Closed`, `Busy`, `InvalidResource`, and `Other` variants |
| `NetError` | `NetError(operation: String, network: Option<Network>, address: Option<String>, kind: NetErrorKind, message: String)` |

The constructors and fields can be matched as ordinary public algebraic data
values. Constructing an `Address` or `Endpoint` does not resolve a name, open a
socket, or validate the contained port or address text.

Code compares a `NetError` kind rather than its explanatory message. This
value-only surface currently produces `InvalidAddress` from the host-port
helpers. The other exported kinds reserve the portable values used by the
planned effectful networking surface; no current `std::net` operation produces
them.

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

## Limits And Errors

Both helpers return `Err(NetError(... InvalidAddress ...))` when a port is less
than zero or greater than 65535. `split_host_port` also returns
`InvalidAddress` when the address has no port, has an empty or non-decimal
port, contains more than one unbracketed colon, or has a bracketed host without
the required closing-bracket-and-port separator. In particular, an IPv6 host
must be bracketed because its unbracketed colons do not identify one port
unambiguously.

The helpers do not validate DNS spelling or the internal syntax of an IP
literal. The module does not yet export `net::IO`, `net::system()`, resolution,
listeners, streams, deadlines, cancellation, or transport adapters. The
remaining work stays in the standard-library networking proposal.

## References

The exported implementation and companion tests are in
`crates/veln-stdlib/veln/net.veln` and
`crates/veln-stdlib/veln/net.test.veln`. The checked command-level example is
under `examples/specification/run/standard-library-network-address-values/`.
