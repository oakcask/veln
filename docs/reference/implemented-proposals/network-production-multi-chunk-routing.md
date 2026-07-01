# Network Production Multi-Chunk Routing

Status: implemented

This record preserves the completed production multi-chunk event routing slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked example under
`../../../examples/specification/run/socket-stream-adapter-production-multi-chunk-routing/case.toml`.

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
handler remains free of transport and channel effects. The slice adds no
effect label, socket handle type, service interface, or HTTP protocol
behavior.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and HTTP/2
transport-adapter behavior beyond the checked production-loopback lifecycle
slices.

## Read When

- Auditing why production multi-chunk stream event routing is no longer active
  proposal work.
- Checking completion evidence before changing production transport chunk
  routing or ordered response projection.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
