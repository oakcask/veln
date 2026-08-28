---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Network Production Listen Connect Lifecycle

This record preserves the completed source-visible production listen/connect
lifecycle slice from [external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is specified by
[names-effects.md](../../specification/names-effects.md),
[names-effects.md](../../specification/names-effects.md),
[execution.md](../../specification/execution.md), and the checked executable
example under
`../../../examples/specification/run/transport-socket-production-listen-connect-lifecycle/`.

## Outcome

The completed slice keeps the existing public `net::listen`, `net::connect`,
`net::accept_or_end`, `net::read_chunk`, `net::write_chunk`,
`net::close_stream`, and `net::close_listener` calls. It adds no transport
abstraction and no effect label beyond the existing coarse `net` boundary.

Under `VELN_NET_RUNTIME = "production-loopback"`, source code can bind a
production-owned listener from a source-visible address value, call
`net::connect` with the same address value while that listener is open, accept
the paired server stream, exchange a one-byte chunk between the source-owned
client and server stream handles, close both stream handles, observe clean
listener end, and close the listener.

The runtime preserves the existing transport failure boundary. Connection,
accept, read, write, stream close, and listener close failures remain runtime
transport failures rather than protocol diagnostics or ordinary source
outcome values.

## Read When

- Auditing why source-visible production listener/client pairing is no longer
  active proposal work.
- Checking completion evidence before changing production-loopback listener,
  connect, accept, or stream ownership behavior.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
