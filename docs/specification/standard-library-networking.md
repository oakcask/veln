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

`Network` has the `Tcp`, `Tcp4`, and `Tcp6` variants. `Address` contains a
network, host string, and integer port. `Endpoint` contains a network and a
resolved host-port string. Their constructors and fields can be matched as
ordinary public algebraic data values. Constructing these values does not
resolve a name, open a socket, or validate the contained port.

`NetError` contains an operation name, optional network, optional address,
`NetErrorKind`, and explanatory message. Code compares the kind rather than
the message. This value-only surface currently produces `InvalidAddress` from
the host-port helpers. The other exported kinds reserve the portable values
used by the planned effectful networking surface.

## Address Values And Host-port Text

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
