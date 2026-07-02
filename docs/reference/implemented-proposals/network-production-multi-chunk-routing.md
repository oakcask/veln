# Network Production Multi-Chunk Routing

Status: implemented

This record preserves the completed production multi-chunk event routing slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-multi-chunk-routing/case.toml`,
`../../../examples/specification/run/socket-stream-adapter-production-multi-event-task-context/case.toml`,
`../../../examples/specification/run/socket-stream-adapter-production-multi-chunk-read-failure-json/case.toml`,
and
`../../../examples/specification/check/socket-stream-adapter-production-multi-chunk-routing-effects/case.toml`.

## Outcome

The completed slice composes existing production-loopback stream ownership,
optional stream reads, channel routing, pure handler calls, and ordered
chunk-list response projection. A checked run case accepts one production
stream, reads more than one configured input chunk through
`net::read_chunk_or_end`, converts each chunk into an ordinary
`StreamInput.Chunk`, routes those values through the existing channel
boundary, and calls a pure handler that receives no `NetStream` handle.

Clean stream end is translated into `StreamInput.End` for the same handler
boundary. The adapter then projects only ordered `SendBytes` response actions
through `net::write_chunks`, closes the stream, and observes clean listener
end. The adapter declares the existing `net` and `concurrency` effects; the
handler remains free of transport and channel effects. The matching static
effect case rejects adapter entry points that omit either `net` or
`concurrency` while leaving the public handler boundary effect-free. The slice
adds no effect label, socket handle type, service interface, or HTTP protocol
behavior.

A companion run case strengthens the same slice by routing each accepted
stream event through the channel boundary and then through an adapter-owned
task helper using `task::spawn_with<Result, Context>`. The context carries
adapter-owned route and trace metadata plus a sequence value. The task helper
owns the `concurrency` boundary and calls a pure event/action handler that
receives no `NetStream`. The checked output confirms that two chunk events and
the clean-end event preserve the same trace identity, ordered sequences,
ordered `net::write_chunks` projection, stream close, and clean listener end.

The matching read-failure case configures the same multi-chunk production
input path and forces `net::read_chunk_or_end` to fail. The failure remains a
runtime transport failure owned by the adapter boundary. The recorded event
sequence stops after production listen and accept, before any chunk routing,
response write, stream close, or clean listener end event.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and HTTP/2
transport-adapter behavior beyond the checked production-loopback lifecycle
slices.

## Read When

- Auditing why production multi-chunk stream event routing is no longer active
  proposal work.
- Checking completion evidence before changing production transport chunk
  routing, task-context trace propagation, forced read-failure ordering, or
  ordered response projection.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
