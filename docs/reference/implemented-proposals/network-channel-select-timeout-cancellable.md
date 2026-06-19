# Network Channel Select Timeout Cancellable

Status: implemented

This record preserves the completed two-receiver cancellable timeout selection
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked examples under
`../../../examples/specification/run/channel-select-timeout-cancellable/` and
`../../../examples/specification/check/channel-select-timeout-cancellable-effects/`.

## Outcome

The completed slice adds
`channel::select_timeout_cancellable(left, right, timeout_ms, token)` with the
same left/right receiver indexes, rotating tie behavior, timeout behavior, and
selected result shape as the existing two-receiver timeout helpers. It returns
`Ok(Some(selected))` when a receiver produces a value, `Ok(None)` when the
timeout elapses or both receivers close before a value is selected, and
`Err(SelectError)` when the supplied `CancelToken` is already cancelled or
becomes cancelled before a receiver wins.

The helper uses the existing `concurrency` and `time` effects. It does not add
channel, socket, task, timer, cancellation, or network-specific effect labels.

## Remaining Work

The broader network integration proposal remains open for production socket
ownership, richer stream routing, richer deadline and cancellation APIs,
channel and task ownership beyond the checked adapter slices, and HTTP/2
transport-adapter work.

## Read When

- Auditing why two-receiver cancellable timeout selection is no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current channel, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
