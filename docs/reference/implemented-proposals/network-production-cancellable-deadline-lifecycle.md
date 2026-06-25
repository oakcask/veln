# Network Production Cancellable Deadline Lifecycle

Status: implemented

This record preserves the completed production-loopback cancellable
deadline-aware adapter lifecycle slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-cancellable-deadline-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-cancellable-deadline-outcomes/`,
and
`../../../examples/specification/check/socket-stream-adapter-production-cancellable-deadline-lifecycle-effects/`.

## Outcome

The completed slice composes the production-loopback runtime path with the
existing cancellable deadline-aware socket calls. Adapter-owned code accepts
deterministic loopback streams with `net::accept_until_cancellable`, reads
deterministic input with `net::read_chunk_until_cancellable`, routes
`ReadChunk` and `ReadEnd` outcomes as ordinary `StreamInput` values through an
existing channel, calls a pure handler, and writes only `SendBytes` response
actions to the owned production stream before closing it.

The lifecycle example drains the owned listener until
`AcceptEnd`, records clean listener end, and then explicitly closes the
listener. The focused outcome example checks production-loopback accept
deadline expiry, accept cancellation, read deadline expiry, and read
cancellation as ordinary adapter decisions instead of runtime transport
failures. The matching effect case requires `net`, `time`, and `concurrency`
at the adapter boundary while keeping the handler free of transport, time, and
channel effects.

This slice adds no effect label and no public runtime call.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, cancellation ownership, scheduler
integration, and HTTP/2 transport-adapter behavior beyond the checked
production-loopback lifecycle slices.

## Read When

- Auditing why this production-loopback cancellable deadline adapter lifecycle
  is no longer active proposal work.
- Checking completion evidence before changing the production transport
  adapter route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
