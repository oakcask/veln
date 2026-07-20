# Network Concurrent Stream Task Drain

Status: implemented

This record preserves the completed concurrent stream task-drain slice from
[external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is
specified by `../../specification/execution.md`,
`../../specification/names-effects.md`, and the executable cases under
`../../../examples/specification/run/socket-stream-adapter-production-concurrent-task-drain/`
and
`../../../examples/specification/check/socket-stream-adapter-production-concurrent-task-drain-effects/`.

## Outcome

The production-loopback adapter accepts streams until clean listener end while
retaining each adapter-owned `NetStream` and its
`Task<Result<HandlerOutput, String>>` in one recursive pending-work shape.
Accepting another stream does not add a field, branch, numbered helper, or
fixed handler arity.

After listener end, the adapter joins pending handler tasks in acceptance
order, projects successful handlers' ordered `SendBytes` actions, and closes
every accepted stream exactly once. A handler-owned `Err` suppresses writes
only for its stream, so later successful streams are still written and
closed. The lifecycle trace proves that acceptance and reads finish before
the listener closes and draining starts, writes retain acceptance order, the
failed stream has no write, and all streams close once.

The adapter uses the existing `net` and `concurrency` effects and the existing
`task::spawn_with<Result, Context>` and `task::join` calls. The handler receives
only an ordinary context record and remains free of transport and concurrency
effects. No public helper, effect label, scheduler behavior, cancellation API,
or service abstraction is added.

## Read When

- Auditing why a fixed two- or three-handler drain is not proposal work.
- Checking acceptance-order projection, per-stream handler failure isolation,
  or stream close ownership for pending handler tasks.

## Skip Unless Needed

- Use the specification pages and executable cases for current behavior.
