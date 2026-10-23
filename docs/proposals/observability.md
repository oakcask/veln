---
role: proposal
update-when: The buffered observation-export queue bound, full-queue admission result, shutdown flushing or drop accounting, final health-summary ordering, or interrupted-shutdown behavior changes.
---

# Buffered Observation Export Lifecycle

The synchronous configured JSON Lines exporter, its checked schema,
metric-cardinality ordering, attribute limits, and bounded failure health are
implemented in the
[current observability specification](../specification/standard-library-observability.md).
This proposal contains only the remaining buffered lifecycle work.

## Boundary

The remaining facility may queue accepted records before a destination write.
It must preserve the synchronous export data contract and must not turn
telemetry delivery into application success or failure. External telemetry
protocol adapters and durable audit guarantees remain outside this proposal.

## Related Work

The OpenTelemetry [Tracing SDK specification](https://opentelemetry.io/docs/specs/otel/trace/sdk/)
defines a finite batch queue, drops spans after that queue reaches capacity,
and requires processor shutdown to include its force-flush behavior. Its flush
and shutdown operations distinguish success, failure, and timeout, and may
prioritize the timeout over completing every export. This provides a concrete
comparison for queue admission and interrupted shutdown, but it does not define
Veln behavior: Veln exports observation kinds beyond spans and must order a
final health summary with its own drop accounting. The proposal therefore
retains the finite-bound and explicit-shutdown questions while leaving their
values and transitions undecided instead of importing OpenTelemetry defaults.

## Shutdown Contract

When a buffered handler shuts down, it must finish processing every record
that it reports as accepted or increment bounded drop accounting before
shutdown returns. A write or flush failure must leave the handled
application's ordinary result unchanged and must not recursively perform
`Observe`.

The lifecycle must define a finite queue bound, the admission result at that
bound, the ordering relationship between queued records and a final health
summary, and the behavior when shutdown itself is interrupted. These values
remain undecided and must be established by executable evidence before they
become current behavior.

## Acceptance Model

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| O14 | Handler shutdown follows buffered emission. | Accepted records are flushed or counted as dropped before shutdown returns. | Deterministic buffered exporter lifecycle case. |

## Verification and Promotion

Companion tests under `crates/veln-stdlib/veln/` must inject scoped Veln test
support for queue admission, destination outcomes, interruption, and health.
Production behavior must use real host effects by default and must not select
fakes through environment variables or system properties. A toolchain case is
appropriate only for a separately identified CLI, backend, or host-lifecycle
contract.

When O14 is implemented, update the current observability specification and
executable evidence, then remove this proposal and its catalog entry.

## Non-goals

- External telemetry-protocol adapters.
- Durable audit guarantees.
- Unrelated changes to observation records or context propagation.
