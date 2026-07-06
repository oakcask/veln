# Network Stream Adapter Routing Helper

Status: implemented

This record preserves the completed source-visible stream adapter routing
helper slice from `../../proposals/network-effect-integration-boundary.md`.
Current behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-routing-helper/case.toml`
and
`../../../examples/specification/check/socket-stream-adapter-routing-helper-effects/case.toml`.

## Outcome

The completed slice adds the source-visible `StreamAdapterAction` ADT with
`SendBytes(ByteChunk)`, `EndStream`, and `Ignore` actions plus
`stream_adapter_drain_actions(stream, handler)`. The helper accepts one
adapter-owned `NetStream` and a pure
`fn(StreamInput) -> List<StreamAdapterAction>` handler. It reads the stream
with `net::read_chunk_or_end`, routes each `StreamInput` value through the
existing channel boundary, preserves ordered handler actions, and writes only
ordered `SendBytes` chunks through `net::write_chunks`.

The helper declares the existing coarse `net` and `concurrency` effects and
does not add socket handle types, fine-grained network labels, deadline
inputs, cancellation inputs, service framework APIs, middleware, or scheduler
abstractions. The checked production-loopback case keeps stream close and
clean listener-end observation in adapter caller code after the helper
returns. The matching effect case rejects adapter entry points that omit
either `net` or `concurrency` while leaving the handler boundary effect-free.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and HTTP/2
transport-adapter behavior beyond the checked helper boundary.

## Read When

- Auditing why the source-visible standard stream adapter routing helper is no
  longer active proposal work.
- Checking completion evidence before changing `StreamAdapterAction`,
  `stream_adapter_drain_actions`, or adapter-owned response projection.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
