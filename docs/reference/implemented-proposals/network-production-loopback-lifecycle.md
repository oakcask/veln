# Network Production Loopback Lifecycle

Status: implemented

This record preserves the completed production-loopback socket lifecycle slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-two-streams/`,
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

Invalid production listen addresses remain runtime transport failures. The
failure example checks the JSON command surface and keeps transport failure
classification separate from protocol diagnostics.

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
