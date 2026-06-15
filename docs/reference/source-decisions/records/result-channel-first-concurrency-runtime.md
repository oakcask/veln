# Discussion Result: Channel-First Concurrency Runtime

Status: implemented
Implementation: channel and task slices implemented

## Picked Question

- Should Veln's implementation model center asynchronous work on
  `async`/`await`, or should it use channel-based coordination on a
  multi-threaded runtime without a global interpreter lock?

## Decision

Use a channel-first concurrency model on a parallel-capable runtime.

Veln should not rely on a GIL, GVL, or equivalent global language lock for
ordinary execution. The reference runtime should be able to run independent
Veln tasks on multiple host threads when the selected backend supports that.
For the first reference backend, this means treating JVM threads or virtual
threads as implementation tools rather than serializing all Veln code behind a
single runtime lock.

The source-level coordination model should prefer structured tasks plus typed
MPSC channels over an `async`/`await`-centered style. `spawn` creates a task.
Normal values and work results move through `Sender<T>` and `Receiver<T>`.
Joining or awaiting a task remains useful for lifecycle, cancellation, and
failure aggregation, but it should not be the default way to express ordinary
data flow between concurrent computations.

Channels should start with a bounded MPSC shape:

```veln
let pair = channel::bounded<String>(32)
```

The sender endpoint may be cloned and sent to multiple producers. The receiver
endpoint is single-consumer in the first model. The default bounded channel
provides backpressure. A rendezvous channel is the capacity-zero case.
Unbounded channels may be added later, but should be explicit rather than the
default.

Channel operations should use existing Veln absence and error conventions.
Receiving from a closed channel returns `Option<T>`. Sending to a channel whose
receiver has been dropped returns `Result<(), SendError>`.

```veln
channel::send(tx, value) -> Result<(), SendError>
channel::recv(rx) -> Option<T>
channel::close(tx) -> ()
```

Concurrency is visible in public effect declarations through the coarse
`concurrency` effect label. Implemented channel and task operations carry that
effect metadata so public APIs cannot silently become concurrent. Future
selection constructs must also carry that effect or its chosen successor label.

## Rationale

Veln's current first-slice decisions already make channel-first concurrency a
natural fit. Bindings and aggregate values are immutable from source code, and
the language does not expose object identity, mutable references, destructors,
or user-controlled allocation. That sharply reduces the amount of shared state
that concurrent tasks can accidentally corrupt.

Adding true parallelism later to a language whose libraries assume pervasive
shared mutable objects is costly. Starting the runtime contract without a
global interpreter lock avoids making single-threaded execution an implicit
compatibility promise. At the same time, using channels as the ordinary
coordination primitive avoids forcing user code into low-level shared-memory
synchronization.

An `async`/`await`-centered design tends to make awaiting values the dominant
composition style. That is useful for callback elimination, but it does not by
itself describe ownership, backpressure, cancellation, or producer-consumer
topology. MPSC channels make those relationships explicit in the program:
producers hold senders, the consumer holds the receiver, and bounded capacity
turns overload into an ordinary scheduling and blocking concern rather than an
unbounded memory-growth concern.

This model also serves the agent-oriented repair loop. Diagnostics can report
which task owns a receiver, which producers can send to it, where a channel may
close, and which public effect boundary introduced concurrency. That is easier
to explain and repair than arbitrary shared mutable state protected by ad hoc
locks.

## Implemented Rules

- The language and reference runtime should not specify or depend on a global
  interpreter lock.
- Source-level concurrent data flow should be expressed primarily with typed
  MPSC channels rather than with general-purpose shared mutable state.
- `Sender<T>` is cloneable for multiple producers; `Receiver<T>` is
  single-consumer in the first model.
- Bounded channels are the default channel constructor. A rendezvous channel is
  a bounded channel with capacity zero.
- Sending returns `Result<(), SendError>` when the receiver is no longer
  available.
- Receiving returns `Option<T>`, with `None` representing closed and drained.
- Channel construction, sender clone, send, receive, and close calls carry the
  coarse `concurrency` effect.
- Task spawn, task join, and task cancellation calls carry the coarse
  `concurrency` effect.
- Public functions whose bodies introduce concurrency must declare that effect
  under the existing public effect boundary rule.
- Locks, atomics, and other shared-memory synchronization primitives are not
  the default coordination model. They may exist later as explicit lower-level
  standard-library tools.

## Open Details

The implemented send and receive operations block a host thread when waiting
for queue capacity, a queued value, a rendezvous transfer, or channel close.
Later task scheduling may replace that with lightweight suspension as long as
the observable source semantics, cancellation behavior, and diagnostics remain
stable.

`select` or an equivalent multi-channel wait form is likely needed, but richer
selection policy should be designed separately because priority, cancellation,
and diagnostic reporting all interact.

Concurrent stdio and test event ordering need a separate decision. Captured
events should avoid host thread identifiers, but source-level task identity or
task paths may become useful once concurrent tests are supported.

Foreign calls and host values may need explicit share-safety metadata before
they can cross task or channel boundaries. Immutable Veln values are safe to
send, but backend-owned resources may require a stricter rule.

## Consequence

Veln keeps a parallel-capable runtime path from the beginning while preserving
a small, repairable source model. Ordinary concurrent programs communicate
through typed bounded channels, public APIs expose concurrency through effects,
and lower-level synchronization remains available for later design without
becoming the default programming style.

## Reference Surface

The current workspace implements a minimal executable bounded-channel slice:
`channel::bounded(capacity)`, `channel::bounded<T>(capacity)`,
`channel::clone(tx)`, `channel::send(tx, value)`, `channel::recv(rx)`,
`channel::select(left, right)`, and `channel::close(tx)` are `concurrency`
effect calls. Public functions and tests that reach these calls must declare
`effects [concurrency]`. It also implements an executable task slice:
`task::spawn(job)`, `task::spawn<T>(job)`,
`task::spawn_with(job, arg)`, `task::spawn_with<T>(job, arg)`,
`task::join(task)`, and `task::cancel(task)` are `concurrency` effect calls.

The implemented constructor infers the item type from an expected
`{tx: Sender<T>, rx: Receiver<T>}` record type, or uses the explicit item type
from `channel::bounded<T>(capacity)`. The runtime supports direct send with
positive-capacity backpressure, sender clone, blocking receive, close on a
single channel pair, and zero-capacity rendezvous transfer between a waiting
sender and receiver. Two-receiver selection returns
`Option<{index: Int, value: T}>`, rotating the first polled receiver when both
receivers are ready in the same poll and returning `None` only after both
receivers are closed and drained. `channel::select_priority` exposes an
explicit left-priority variant for callers that need deterministic receiver
preference. The task runtime starts zero-argument callables on JVM threads,
freezes task results before they cross the task boundary, joins with
`Result<T, JoinError>`, and treats cancellation as a cooperative interruption
request. Result-returning channel selection variants report cooperative
cancellation separately from closed or timed-out selection.
