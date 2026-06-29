# Network Production Owner-Drain Lifecycle

Status: implemented

This record preserves the completed production owner-drain cancellable
deadline lifecycle slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-owner-drain-cancellable-deadline-lifecycle/`
and
`../../../examples/specification/check/socket-stream-adapter-production-owner-drain-effects/`.

## Outcome

The completed slice composes existing production-loopback stream ownership,
cancellable deadline-aware accept/read outcomes, adapter-owned cancellation
authority, channel-routed ordinary handler input, and ordered chunk-list
response projection. Adapter-owned code creates a `CancelOwner`, passes only
observer `CancelToken` values to `net::accept_until_cancellable`,
`net::read_chunk_until_cancellable`, and channel-routing code, calls a pure
handler with ordinary `StreamInput` values and explicit state, and projects
only ordered `SendBytes` actions through `net::write_chunks`.

The executable run case drains configured production streams until clean
listener end, closes each owned stream after response projection, closes the
listener during cleanup, captures the ordered client-observed bytes, and
checks a second adapter path where owner-requested cancellation makes
cancellable accept return `AcceptCancelled` as an ordinary outcome. It also
checks a path that accepts one production stream, routes one ordinary stream
event through the channel and task handler boundary, requests owner
cancellation, then observes the next cancellable read as ordinary
`ReadCancelled` before another handler route continues. The effect case
requires the existing `net`, `time`, and `concurrency` labels at the adapter
boundary while keeping the handler boundary free of transport, time, and
channel effects.

Forced host failures on the same production accept, read, write, and close
boundaries remain runtime transport failures covered by the adjacent
production-loopback and outbound write-failure cases. This slice adds no
effect label, public socket call, task arity helper, route-count fixture, or
service framework.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and HTTP/2
transport-adapter behavior beyond the checked production-loopback lifecycle
slices.

## Read When

- Auditing why this composed production owner-drain adapter lifecycle is no
  longer active proposal work.
- Checking completion evidence before changing production transport adapter
  ownership, cancellation, or ordered write projection.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
