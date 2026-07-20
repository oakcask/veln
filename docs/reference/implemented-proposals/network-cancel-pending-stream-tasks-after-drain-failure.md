# Network Cancel Pending Stream Tasks After Drain Failure

Status: implemented

This record preserves the completed fail-fast pending-task cleanup slice from
[external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is
specified by `../../specification/execution.md`,
`../../specification/names-effects.md`, and the executable cases under
`../../../examples/specification/run/socket-stream-adapter-production-cancel-pending-after-failure/`
and
`../../../examples/specification/check/socket-stream-adapter-production-cancel-pending-after-failure-effects/`.

## Outcome

The production-loopback adapter retains accepted `NetStream` values and their
`Task<Result<HandlerOutput, String>>` handles in the existing recursive
pending-work shape until clean listener end. It closes the listener before it
starts pending-task cleanup.

The adapter joins tasks in acceptance order until the first handler-owned
`Err` or task-join failure. Successful writes completed before that failure
remain visible. The failed stream is closed without a response write. Every
later task is passed to `task::cancel` and then `task::join`, every retained
stream is closed exactly once, and no later response bytes are projected.
Cancellation join outcomes are cleanup results and do not stop recursive
reclamation.

The adapter uses the existing `net` and `concurrency` effects. Its ordinary
handler receives no transport or task handles and remains effect-free. This
slice adds no scheduler API, task-group API, effect label, service abstraction,
fixed stream count, or new cancellation primitive.

## Read When

- Auditing fail-fast cancellation, task reclamation, or close ownership after
  a pending handler failure.
- Distinguishing this adapter policy from the non-cancelling drain that
  isolates one handler failure and continues later successful writes.

## Skip Unless Needed

- Use the specification pages and executable cases for current behavior.
