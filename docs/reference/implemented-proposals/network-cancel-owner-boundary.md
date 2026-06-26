# Network Cancel Owner Boundary

Status: implemented

This record preserves the completed adapter-owned cancellation owner slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-cancel-owner-lifecycle/`,
`../../../examples/specification/run/transport-cancel-owner-observer-only-json/`,
and
`../../../examples/specification/check/socket-stream-adapter-cancel-owner-lifecycle-effects/`.

## Outcome

The completed slice adds a source-visible `CancelOwner` that can expose an
observer `CancelToken` through `time::cancel_token_from(owner)` while keeping
cancellation authority with adapter cleanup through
`time::cancel_owned(owner)`. Existing `CancelToken` observer calls remain the
compatibility boundary for cancellable waits, channel selection, and socket
operations, and direct tokens created by `time::cancel_token` can still be
cancelled with `time::cancel(token)`.

The adapter lifecycle example keeps the `CancelOwner` in cleanup code, passes
only the observer token to routing, wait, and cancellable socket-read code,
requests cancellation through the owner, and then observes `WaitCancelled` and
`ReadCancelled` as ordinary values before closing the stream and listener. The
observer-only runtime case keeps direct `time::cancel(token)` on an
owner-derived observer token on the runtime-failure surface. The matching
effect case requires the existing `net`, `time`, and `concurrency` labels at
the adapter boundary while keeping the handler free of transport effects.

This slice adds no effect label, does not change the existing `CancelToken`
wait/status API, does not add a service framework or structured-concurrency
model, and does not make the pure protocol core observe cancellation handles.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and deadline or
cancellation APIs beyond the current owner/token split.

## Read When

- Auditing why adapter-owned cancellation authority is no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
