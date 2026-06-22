# Network Production Loopback Lifecycle

Status: implemented

This record preserves the completed production-loopback socket lifecycle slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-two-streams/`,
`../../../examples/specification/run/socket-stream-adapter-production-drain-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-drain-read-failure-json/`,
`../../../examples/specification/run/socket-stream-adapter-production-close-failure-json/`,
`../../../examples/specification/run/transport-socket-production-two-streams/`,
and
`../../../examples/specification/run/transport-socket-production-listen-failure-json/`.

## Outcome

The completed slice lets executable specification cases opt into a host-owned
loopback transport path by setting `VELN_NET_RUNTIME` to
`production-loopback`. The path uses the existing public `net::listen`,
`net::accept`, `net::accept_or_end`, `net::read_chunk`,
`net::read_chunk_or_end`, `net::write_chunk`, and `net::close_stream` calls.
It adds no public call and no effect label beyond the existing coarse `net`
and `concurrency` declarations used by the adapter lifecycle.

The checked lifecycle binds a loopback listener, accepts streams, reads client
bytes into ordinary `StreamInput` values or direct `ByteChunk` values, routes
adapter-owned values through the existing channel boundary where applicable,
calls a pure handler where applicable, writes ordered response bytes to each
accepted stream, and closes each stream. The runtime captures the bytes
observed by loopback clients, so the examples pin both source-visible output
and host transport output. A checked two-stream case also pins clean listener
end after the planned accepted streams are exhausted. The checked two-stream
adapter case additionally proves that two accepted production streams can use
the same ordinary `StreamInput` handler/action boundary independently, with
only ordered `SendBytes` actions projected to socket writes before each stream
is closed.

The listener-drain adapter slice uses the same public calls and effect
declarations without hard-coding a separate accept path for each stream. It
recursively accepts configured production streams until `net::accept_or_end`
reports clean listener end, routes every accepted stream through the existing
ordinary handler/action boundary, writes only ordered `SendBytes` actions,
closes each stream, and captures all client-observed byte sequences.

Invalid production listen addresses and forced production close failures
remain runtime transport failures. The failure examples check the JSON command
surface and keep transport failure classification separate from protocol
diagnostics. The close-failure case also pins that adapter-routed ordered
writes happen before the failed close and that no successful close event is
recorded after the forced failure. The listener-drain read-failure case pins a
forced production read failure after accept and before response writes or
stream close.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs beyond this deterministic loopback lifecycle, richer stream
routing, richer deadline and cancellation APIs, channel and task ownership
beyond the checked adapter slices, and HTTP/2 transport-adapter work.

## Read When

- Auditing why the first production socket lifecycle is no longer active
  proposal work.
- Checking completion evidence before changing the production transport
  adapter route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
