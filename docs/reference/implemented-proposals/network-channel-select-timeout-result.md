# Network Channel Select Timeout Result

Status: implemented

This record preserves the completed two-receiver timeout-result selection
slice from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked examples under
`../../../examples/specification/run/channel-select-timeout-result/` and
`../../../examples/specification/check/channel-select-timeout-result-effects/`.

## Outcome

The completed slice adds
`channel::select_timeout_result(left, right, timeout_ms)` with the same
left/right receiver indexes, rotating tie behavior, timeout behavior, and
selected result shape as the existing two-receiver timeout helpers. It returns
`Ok(Some(selected))` when a receiver produces a value, with the same tie
selection as `channel::select_timeout`. It returns `Ok(None)` when the timeout
elapses or both receivers close before a value is selected, and
`Err(SelectError)` when a runtime selection interruption reaches the fallible
selection boundary.

The helper uses the existing `concurrency` effect and does not require the
`time` effect. Its timeout argument remains an `Int` source type boundary.
It does not add channel, socket, task, timer, cancellation, or network-specific
effect labels.

## Read When

- Auditing why two-receiver timeout-result selection is no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current channel, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
