# Network Production Two-Stream Multi-Cycle Routing

Status: implemented

This record preserves the completed production two-stream multi-cycle routing
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked example under
`../../../examples/specification/run/socket-stream-adapter-production-two-stream-multi-cycle-routing/case.toml`.

## Outcome

The completed slice composes existing production-loopback listener ownership,
accepted stream ownership, optional stream reads, channel routing, pure handler
calls, ordered chunk-list response projection, and stream close. A checked run
case accepts two production streams from one listener. For each stream, adapter
code reads more than one configured input chunk through
`net::read_chunk_or_end`, converts each chunk into an ordinary
`StreamInput.Chunk`, routes those values through the existing channel boundary,
calls a pure handler that receives no `NetStream`, and observes clean end as
`StreamInput.End`.

The adapter projects only ordered `SendBytes` response actions through
`net::write_chunks`, closes each owned stream, and observes clean listener end
after the second stream. The net-event fixture output records listener accept
order, per-stream read order, adapter-owned write order, stream closes, and the
final clean listener end. The client-write fixture records the bytes returned
to each stream in stream order.

The slice adds no effect label, socket handle type, service interface, TLS,
ALPN, HTTP routing, or direct socket access from application handlers. The
static effect boundary remains the same as the production multi-chunk routing
slice: adapter code owns the existing `net` and `concurrency` effects, while
the handler boundary remains free of transport and channel effects.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and HTTP/2
transport-adapter behavior beyond the checked production-loopback lifecycle
slices.

## Read When

- Auditing why two-stream multi-cycle production routing is no longer active
  proposal work.
- Checking completion evidence before changing production-loopback stream
  ownership, repeated read routing, ordered response writes, or clean listener
  end behavior.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked executable example for current
  behavior.
