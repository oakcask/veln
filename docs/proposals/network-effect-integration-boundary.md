# Network Effect Integration Boundary

Status: proposed

This proposal tracks transport integration that remains beyond the checked
socket, channel, task, deadline, and cancellation boundaries. Current behavior
is specified by `../specification/names-effects.md` and
`../specification/execution.md`; executable routes are listed by
`../specification/examples.md`.

## Problem

The HTTP/2 core is pure, while production integration needs host-owned sockets,
deadlines, task lifecycle, cancellation, and routing. The remaining design
must preserve that separation without turning schema or protocol code into a
network runtime.

## Remaining Scope

- production socket ownership beyond deterministic loopback fixtures
- adapter ownership of accepted streams and pending stream tasks through
  success, failure, cancellation, and reclamation
- richer production read, write, accept, connect, and close APIs when an
  adapter use case cannot be expressed by the current boundaries
- translation between transport chunks and pure `StreamInput` events, and
  between protocol output chunks and ordered host writes
- transport error context for host failures without reclassifying them as peer
  protocol errors

Deadline and cancellation work is complete for this proposal at the current
`Deadline`, `CancelToken`, cancellation-owner, status, wait-outcome, and
cancellable socket-operation boundary. A richer timer, scheduler, or
cancellation capability needs a concrete adapter use case and a separate
runtime proposal.

## Design Constraints

- Pure protocol functions do not perform `net`, `time`, or `concurrency`
  effects.
- Adapter functions declare the coarse effects required by their host calls.
- Clean stream end is an ordinary adapter-observable outcome; host I/O failure
  remains a runtime or adapter failure.
- A closed stream may cause the pure core to report an incomplete protocol
  fact, but transport context remains related context rather than replacing
  the schema, codec, or protocol diagnostic.
- Handler context is one ordinary source value. More positional task-helper
  arities are not remaining work.
- The two-, three-, four-route, and receiver-list channel examples already
  establish routing shape. Another route-count fixture is not remaining work.

## Non-Goals

- TLS or ALPN
- moving socket or clock access into the pure protocol core
- finer-grained effect labels without an API that requires them
- HTTP application routing
- reopening completed deadline or cancellation slices

## Completion Criteria

- Executable cases cover ownership, ordered output, clean end, host failure,
  cancellation, and task reclamation for each selected production adapter
  capability.
- Effect checking covers every new compiler-known host call.
- The smallest matching specification pages describe the observable boundary.
- Completed slices move to implemented-proposal records and do not accumulate
  here.
