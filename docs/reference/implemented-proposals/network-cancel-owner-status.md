# Network Cancel Owner Status

Status: implemented

This record preserves the completed cancellation-owner status query slice from
[external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-cancel-owner-status/` and
`../../../examples/specification/check/transport-cancel-owner-status-effects/`.

## Outcome

The completed slice adds
`time::is_cancelled_owner(owner: CancelOwner) -> Bool effects [time]` as a
source-visible standard-library call. Adapter-owned code can inspect its own
`CancelOwner` directly before and after `time::cancel_owned(owner)` without
creating an observer `CancelToken`.

The executable run case observes `active`, cancels through the owner, and then
observes `cancelled`. The focused effect case keeps the boundary explicit: a
public function that calls `time::is_cancelled_owner` must declare `time`.

This slice adds no effect label, does not change `CancelToken`,
`time::cancel_token_from`, `time::cancel_owned`, `time::cancel(token)`, or
`time::is_cancelled(token)` behavior, and does not make the pure protocol core
depend on cancellation handles.

## Read When

- Auditing why direct `CancelOwner` status queries are no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
