# Network Effect Integration Boundary

Status: proposed

This proposal tracks two remaining transport integration targets beyond the
checked loopback socket and adapter lifecycle boundaries. Current behavior is
specified by `../specification/names-effects.md` and
`../specification/execution.md`; executable routes are listed by
`../specification/examples.md`.

## Problem

The HTTP/2 core is pure, while the current opt-in production transport remains
a deterministic loopback harness. Host failures also cross the runtime
boundary primarily as messages rather than structured transport facts. The
remaining design must address those two boundaries without turning schema or
protocol code into a network runtime.

## Target A: External Production Socket Runtime

Add an explicit production runtime mode that uses host sockets independently of
the deterministic loopback client and in-memory fallback.

- `net::listen` binds the requested endpoint or returns a transport failure; it
  does not start a synthetic client and does not silently replace a failed bind
  with an in-memory listener.
- `net::connect` may connect to an external endpoint that was not registered by
  the current process; it does not synthesize an in-memory stream for an
  unknown endpoint.
- Existing `NetListener` and `NetStream` ownership, clean-end, half-close,
  deadline, cancellation, address inspection, and stale-handle rules apply to
  the external path without exposing host socket objects to source code.
- The deterministic fixture and production-loopback modes remain available for
  executable specification cases and do not change semantics.

## Target B: Structured Transport Failure

Replace message-only host I/O failures at public network boundaries with a
structured transport failure payload containing:

- the operation: listen, connect, accept, read, write, shutdown, stream close,
  or listener close
- the source-visible local or peer endpoint when known
- the owned listener or stream identity when known
- the lifecycle phase and whether any input, output, or ownership transition
  committed before failure
- a stable host-failure category, with platform-specific cause text retained as
  related context rather than used as the category

Transport failures remain runtime or adapter failures. They must not be
reclassified as schema, codec, HPACK, or peer HTTP/2 protocol failures.

## Completed Boundaries Excluded From Scope

Deadline and cancellation work is complete for this proposal at the current
`Deadline`, `CancelToken`, cancellation-owner, status, wait-outcome, and
cancellable socket-operation boundary. A richer timer, scheduler, or
cancellation capability needs a concrete adapter use case and a separate
runtime proposal.

Accepted-stream ownership, pending task retention, success and failure drain,
cancellation, reclamation, channel-routed `StreamInput`, ordered host writes,
clean end, and adapter effect declarations are also complete at the checked
loopback boundary. More task-helper arities, route-count examples, or another
same-shaped lifecycle fixture are not remaining work.

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
- richer read, write, accept, connect, or close calls without a concrete
  external-adapter use case that the current calls cannot express
- HTTP application routing
- reopening completed deadline or cancellation slices

## Target A Completion Criteria

- A checked external client can connect without an in-process listener binding,
  and a checked external listener can accept without a synthetic loopback
  client.
- Bind and connect failures do not fall back to in-memory transport.
- Executable cases cover ownership, ordered output, clean end, half-close,
  cancellation, and reclamation through the external path.
- Existing source calls and coarse effects are reused unless the implementation
  demonstrates a concrete missing capability.

## Target B Completion Criteria

- Every public host network operation projects the structured transport payload
  in human and JSON output.
- Tests cover endpoint context, ownership phase, committed-state facts, stable
  categories, and platform cause text as related context.
- Adapter and pure protocol failures retain their existing classification and
  precedence when transport context is attached.

For each target, update the smallest matching specification pages after
executable evidence exists. Archive a completed target under the implemented
proposal records and remove it from this page. Archive this page when both
targets are complete.
