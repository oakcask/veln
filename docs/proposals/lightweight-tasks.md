---
role: proposal
update-when: The controlled lightweight-task baseline, candidate measurements, adoption budgets, or benchmark-runner availability change.
---

# Lightweight Tasks: Controlled Adoption Gate

The JVM runtime now uses one unpooled virtual thread per Veln task and gives
each run and reusable test entry a distinct invocation lifetime. Current
behavior, limits, and executable command evidence are owned by the
[effects](../specification/effects.md#concurrency-calls),
[execution](../specification/execution.md#lightweight-task-execution-and-invocation-lifetime),
[run](../specification/command-run.md#analysis-and-execution-gates), and
[test](../specification/command-test.md#selection-and-jobs) specifications.

One adoption requirement remains incomplete: the controlled platform-thread
baseline versus virtual-thread candidate comparison. Functional progress,
entry lifetime, startup rollback, virtual-worker identity, bounded carrier
progress, candidate allocation, and repeated registration release have JVM
integration coverage. Those checks do not replace the controlled cost and
retained-heap comparison below.

## Remaining Outcome

Run prebuilt platform-thread baseline and virtual-thread candidate programs
with the same Java runtime, host, JVM flags, heap limit, and Veln workload.
Build and analysis time are outside each runtime measurement. Both programs
must first produce the same functional result.

The comparison must cover:

| Workload | Required measurements |
| --- | --- |
| Parked task populations | OS threads, RSS, heap, creation and completion time, and first resource failure. |
| Rendezvous and bounded-channel pipelines | Throughput, CPU time, and end-to-end latency distribution. |
| Idle selects with occasional messages | Idle CPU, wakeups, and delivery latency. |
| Loopback connections | Throughput, latency, OS threads, descriptors, and selector counts. |
| CPU tasks mixed with short jobs | Throughput and short-job latency under carrier saturation. |
| Repeated completed-task batches | Live heap after application handles are released and collection runs. |

Use JFR pinning events only as a diagnostic for carrier occupancy. Distinguish
OS platform-thread count from Java virtual-thread count. Keep the existing
channel monitor, polling-select, per-operation socket selector, and bounded
DNS-worker algorithms unchanged so their costs remain separable.

## Controlled Budgets

These are project adoption choices, not public task quotas or general latency
guarantees.

| Concern | Gate |
| --- | --- |
| Runner bounds | Prebuilt programs; 4 GiB process-tree memory ceiling, 512 MiB Java heap, and 180 seconds per JVM run. Stop a population ladder after its first resource failure. |
| Samples and noise | Discard one warm-up and collect seven independent measured JVM runs per configuration in alternating order. Report medians, ranges, and per-run latency p95. A timeout or allocation failure remains a failed sample. A range greater than 20% of its median is inconclusive. |
| Waiting-task allocation | Park 256, 1,024, 4,096, and 16,384 jobs with carrier parallelism and maximum pool size fixed to two. Candidate OS-thread growth between 256 and 16,384 is at most eight, and the candidate completes 16,384 within the runner bounds. |
| Common-population cost | At 1,024 jobs both implementations complete. Candidate median peak RSS does not exceed baseline, and candidate worker OS-thread count is at most one quarter of baseline. |
| Runtime regressions | At 1,024 jobs, channel-pipeline and loopback throughput are at least 80% of baseline. Delivery-latency p95 is at most 125% of baseline under the same offered load. Publish workload source, payload sizes, counts, and offered rate before collecting either result. |
| Reclamation | Run ten batches of 1,024 spawn, complete, and release operations. The last five collected live-heap samples remain within a 10% band of their median. Runtime inspection also observes zero completed registrations and no retained released owners. |
| Diagnostics | Report polling-select idle CPU and wakeups, descriptor and selector counts, CPU-saturation latency, and JFR pinning events without an adoption ratio. A resource leak or functional failure still fails its correctness gate. |

Uncontrolled CI must not gate wall-clock ratios. It continues to own bounded
functional, carrier-progress, allocation-shape, startup-rollback, and lifetime
regressions.

## Current Blocker

The local guarded runner cannot establish the common-population baseline. A
platform-thread process reaches the host native-thread resource limit before
1,024 workers even with a reduced Java thread stack, while the candidate
completes 1,024 parked tasks and repeated batches. A resource failure is a
failed baseline sample and cannot be omitted or treated as a successful
comparison point. Complete the gate on a controlled runner that can finish the
required platform baseline without relaxing the specified memory ceiling.

Remove this page and its Ready catalog entry only after the controlled artifact
passes every budget above. Preserve no bounded result snapshot under `docs/`;
the benchmark source, machine-readable artifact, and current specifications
must own the completed evidence.
