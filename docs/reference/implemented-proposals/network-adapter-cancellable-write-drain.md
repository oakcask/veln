# Network Adapter Cancellable Write-Drain

Status: implemented

This record preserves the completed adapter-level cancellable write-drain
helper slice from `../../proposals/network-effect-integration-boundary.md`.
Current behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-cancellable-write-drain/`,
`../../../examples/specification/run/socket-stream-adapter-cancellable-write-drain-deadline/`,
`../../../examples/specification/run/socket-stream-adapter-cancellable-write-drain-cancelled/`,
and
`../../../examples/specification/check/socket-stream-adapter-cancellable-write-drain-effects/`.

## Outcome

The completed slice adds
`stream_adapter_drain_actions_until_cancellable(stream, handler, deadline,
token)` as a source-visible standard helper over one adapter-owned
`NetStream`, a pure `fn(StreamInput) -> List<StreamAdapterAction>` handler,
a source-visible `Deadline`, and a source-visible `CancelToken`.

The helper uses the same channel-routed `StreamInput` boundary and ordered
handler action preservation as `stream_adapter_drain_actions`. It projects only
ordered `SendBytes(ByteChunk)` actions into the write path and delegates the
write outcome to `net::write_chunks_until_cancellable`. Full completion
returns `WriteCompleted`; deadline expiry before all projected chunks are
written returns `WriteDeadlineExpired`; cancellation before all projected
chunks are written returns `WriteCancelled`. Host write failures remain
runtime transport failures.

The boundary keeps the existing coarse `net`, `time`, and `concurrency`
effects. The handler stays free of transport, deadline, and cancellation
effects, and the slice does not add socket syntax, new effect labels, service
interfaces, middleware, or route-count-only fixtures.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, cancellation ownership, scheduler
integration, and HTTP/2 transport-adapter behavior beyond the checked helper
boundary.

## Read When

- Auditing why the adapter-level cancellable write-drain helper is no longer
  active proposal work.
- Checking completion evidence before changing
  `stream_adapter_drain_actions_until_cancellable`,
  `StreamAdapterAction`, or cancellable adapter-owned response projection.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
