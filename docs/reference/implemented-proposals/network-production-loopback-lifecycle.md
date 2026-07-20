# Network Production Loopback Lifecycle

Status: implemented

This record preserves the completed production-loopback socket lifecycle slice
from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-two-streams/`,
`../../../examples/specification/run/socket-stream-adapter-production-drain-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-drain-read-failure-json/`,
`../../../examples/specification/run/socket-stream-adapter-production-deadline-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-accept-until-failure-json/`,
`../../../examples/specification/run/socket-stream-adapter-production-read-until-failure-json/`,
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

The deadline-aware production adapter slice uses the same deterministic
loopback event recording with the existing `net::accept_until` and
`net::read_chunk_until` calls. The adapter accepts a production stream before
the deadline, reads until clean stream end becomes `None`, routes ordinary
`StreamInput` values through the handler/action boundary, writes the ordered
response bytes, closes the stream, and observes clean listener end through a
following deadline-aware accept. This slice adds no public call and preserves
the coarse `net`, `time`, and `concurrency` effect labels.

Invalid production listen addresses and forced production close failures
remain runtime transport failures. The failure examples check the JSON command
surface and keep transport failure classification separate from protocol
diagnostics. The close-failure case also pins that adapter-routed ordered
writes happen before the failed close and that no successful close event is
recorded after the forced failure. The listener-drain read-failure case pins a
forced production read failure after accept and before response writes or
stream close. The deadline-aware accept and read failure cases pin the same
runtime transport-failure surface for forced production failures through
`net::accept_until` and `net::read_chunk_until`.

## Read When

- Auditing why the first production socket lifecycle is no longer active
  proposal work.
- Checking completion evidence before changing the production transport
  adapter route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
