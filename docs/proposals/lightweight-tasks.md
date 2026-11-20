---
role: proposal
update-when: The JVM virtual-thread candidate, controlled lightweight-task comparison, adoption budgets, or adoption verdict changes.
---

# Lightweight Task Adoption

Adopt one unpooled Java virtual thread per Veln task only after the controlled
comparison satisfies every fixed D6 gate. The current production worker remains
a non-daemon platform thread because the checked comparison fails the channel
latency and complete-noise gates.

The implemented invocation lifetime, atomic admission, completion tracking,
cancellation, cleanup, handler inheritance, and task result contracts are owned
by the [effects](../specification/effects.md#concurrency-calls) and
[execution](../specification/execution.md#lightweight-task-execution-and-invocation-lifetime)
specifications. This proposal owns only the unresolved worker adoption and its
acceptance evidence.

## Scope And Boundaries

The candidate uses `Thread.startVirtualThread` once for each admitted task. It
keeps ordinary synchronous Veln calls and the existing task handles. It does not
pool virtual threads.

Keep the current channel monitor, polling select, per-operation socket selector,
and bounded DNS-worker algorithms. Do not add structured concurrency, task
groups, sibling cancellation, scheduler APIs, public task quotas, async state
machines, stronger CPU fairness, new wait tie rules, or another backend.

Java virtual threads preserve sequential stacks while the JDK schedules them
over carrier threads. [JEP 491](https://openjdk.org/jeps/491) permits virtual
threads to unmount during monitor acquisition and `Object.wait()`, which is
required by the retained channel implementation. Native calls and other host
operations can still occupy carriers. The proposal does not promise universal
nonblocking host calls or bounded CPU-task fairness.

## Controlled Comparison

Compare a platform-thread production build and a virtual-thread candidate built
from the same generated runtime. Change only the worker constructor. Use the
same JDK, JVM flags, heap limit, source workload, and host for both variants.
Build and analysis time are outside the measurement.

The checked `benchmarks/lightweight-tasks/result.json` artifact records seven
independent measured runs per variant after one discarded warm-up.
Variant order alternates. The harness verifies received channel identity and
order, exact loopback payload bytes, completed registrations, and owner
reclamation before cost gates are evaluated.

| Concern | Required gate |
| --- | --- |
| Bounded runner | Each JVM has a 512 MiB heap, a 4 GiB externally observed RSS ceiling, and a 180 second limit. |
| Samples and noise | Every governed metric has a range no greater than 20% of its median. A timeout, allocation failure, or inconclusive metric blocks adoption. |
| Waiting-task allocation | Park 256, 1,024, 4,096, and 16,384 tasks with carrier parallelism and maximum pool size fixed to two. Candidate OS-thread growth from 256 to 16,384 is at most eight, and 16,384 completes. |
| Carrier progress | With candidate carrier parallelism and maximum pool size fixed first to one and then to two, finite channel rendezvous and full-buffer schedules that include timer waits and nested task joins complete within the bounded harness timeout. |
| Common-population cost | At 1,024 tasks both variants complete. Candidate median peak RSS does not exceed production, and candidate worker OS threads are at most one quarter of production. |
| Runtime cost | Channel and loopback throughput are at least 80% of production. Their delivery-latency p95 medians are at most 125% of production under the fixed offered load. |
| Reclamation | Ten batches of 1,024 completed and released tasks leave the last five live-heap samples within 10% of their median, zero completed registrations, and zero retained released owners. |
| Functional and lifecycle results | Both variants produce the identity-sensitive expected outcomes, candidate cancellation is observed under CPU saturation, and the existing bounded lifetime and host-integration evidence passes. |

Polling-select CPU, wakeups, descriptors, selector operations, CPU-saturation
latency, and JFR pinning events remain reported diagnostics. A functional or
resource failure still fails its corresponding gate.

## Current Blocking Evidence

The corrected comparison remains failed. The stable rendezvous-channel p95
medians are 84,020 ns for production and 142,404 ns for the candidate. The
stable bounded-channel medians are 58,107 ns and 111,067 ns. Both candidate
ratios exceed the fixed 125% ceiling. The complete noise rule is also
inconclusive because several governed metrics exceed their allowed range.

The allocation, common-population, throughput, loopback latency, reclamation,
lifecycle, and identity-sensitive functional gates pass in the same artifact.
Those results do not override a failed required gate. Production adoption and
required-CI artifact validation remain blocked until one complete recollection
passes every gate without changing the workload, budgets, or retained wait
algorithms merely to obtain success.

## Completion

When one corrected controlled comparison passes all gates, switch the production
constructor to the virtual-thread candidate, require the passing artifact in CI,
and update the current execution specification. Then remove this proposal and
its Ready catalog entry; current specifications, executable examples, backend
tests, and the checked benchmark become the behavior authorities.
