---
role: proposal
update-when: The JVM virtual-thread candidate, controlled lightweight-task comparison, adoption budgets, adoption verdict, or checked adoption evidence changes.
---

# Lightweight Task Adoption

Adopt one unpooled Java virtual thread per Veln task only after the controlled
comparison satisfies every fixed D6 gate. The current production worker remains
a non-daemon platform thread because the checked comparison fails both channel
latency gates and the complete-noise gate.

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

## Alternatives And Related Work

| Alternative and inspected source | Finding and applicability | Design consequence |
| --- | --- | --- |
| Keep platform workers | The current runtime creates one OS-backed worker for every task, including waiting tasks. | Retain this implementation as the controlled comparison baseline. It keeps the allocation limit that this proposal addresses. |
| Use a fixed platform-worker pool | With Veln's synchronous blocking calls, every worker can wait for children or channel peers that have no worker available. This is an inference from the current implementation, not a measured prototype result. | A fixed pool alone does not meet the finite-progress requirement. A pool alternative needs a separate suspension mechanism with its own evidence. |
| Use Java virtual threads: [JEP 444](https://openjdk.org/jeps/444), [JEP 491](https://openjdk.org/jeps/491), the [JDK thread model](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/Thread.html), and the [virtual-thread guide](https://docs.oracle.com/en/java/javase/25/core/virtual-threads.html) | Virtual threads retain sequential Java stacks while the JDK multiplexes waiting tasks. JEP 491 covers the monitor suspension used by Veln channels. Native calls and other host work can still occupy carriers. Virtual threads are daemon threads. | This is the selected JVM mechanism. Invocation lifetime must keep the platform entry launcher alive, and the D6 comparison must establish the project-specific cost before adoption. |
| Use Go-style runtime scheduling: [runtime implementation guidance](https://github.com/golang/go/blob/go1.24.0/src/runtime/HACKING.md) and [runtime release notes](https://go.dev/doc/go1.14#runtime) | Go owns goroutine stacks, OS-thread scheduling, and asynchronous preemption. Those mechanisms are properties of the Go runtime and compiler, not evidence about the measured JVM candidate. | Retain lightweight multiplexing as the goal. Do not promise Go-equivalent CPU fairness or import its stack machinery into synchronous JVM calls. |
| Lower tasks to async state machines: [Tokio task documentation](https://docs.rs/tokio/1.54.1/tokio/task/) | Tokio tasks schedule cooperatively, blocking work needs separate treatment, and started blocking jobs generally cannot be aborted. This differs from Veln's synchronous execution and cooperative cancellation model. | Keep this as a possible future-backend design. It would require specified suspension across calls, handlers, cleanup, and host effects; it is not a substitute for correcting the selected JVM candidate. |

The controlled comparison measures only the platform-worker baseline and the
one-constructor virtual-thread candidate. Its failed latency result is not
evidence for or against the unmeasured pool, Go-runtime, or async-state-machine
alternatives.

## Controlled Comparison

Compare a platform-thread production build and a virtual-thread candidate built
from the same generated runtime. Change only the worker constructor. Use the
same JDK, JVM flags, heap limit, source workload, and host for both variants.
Build and analysis time are outside the measurement.

The checked `benchmarks/lightweight-tasks/result.json` artifact records seven
independent measured runs per variant after one discarded warm-up. Variant
order alternates. Each sample separates parked-task and common-cost JVM phases
with identical runtime identity and arguments. The harness verifies received
channel identity and order, exact loopback payload bytes, completed
registrations, and owner reclamation before cost gates are evaluated.

| Concern | Required gate |
| --- | --- |
| Bounded runner | Each JVM has a 512 MiB heap, a 4 GiB externally observed RSS ceiling, and a 180 second limit. |
| Samples and noise | Every governed metric has a range no greater than 20% of its median. A timeout, allocation failure, or inconclusive metric blocks adoption. |
| Waiting-task allocation | Park 256, 1,024, 4,096, and 16,384 tasks with carrier parallelism and maximum pool size fixed to two. Candidate OS-thread growth from 256 to 16,384 is at most eight, and 16,384 completes. |
| Carrier progress | With candidate carrier parallelism and maximum pool size fixed first to one and then to two, finite channel rendezvous and full-buffer schedules that include timer waits and nested task joins complete within the bounded harness timeout. |
| Common-population cost | At 1,024 tasks both variants complete. Candidate median peak RSS does not exceed production, and candidate worker OS threads are at most one quarter of production. |
| Runtime cost | Channel and loopback throughput are at least 80% of production. Their delivery-latency p95 medians are at most 125% of production under the fixed offered load. |
| Loopback resources | Linux process-descriptor measurement is available before and after the fixed loopback workload. Every accepted and connected stream and the listener close, and every measured run returns to zero descriptor growth. Selector-operation count remains diagnostic. |
| Reclamation | Ten batches of 1,024 completed and released tasks leave the last five live-heap samples within 10% of their median, zero completed registrations, and zero retained released owners. |
| Functional and lifecycle results | Both variants produce the identity-sensitive expected outcomes, candidate cancellation is observed under CPU saturation, and the existing bounded lifetime and host-integration evidence passes. |

Polling-select CPU, wakeups, selector-operation count, CPU-saturation latency,
and JFR pinning events remain reported diagnostics. The measured descriptor
delta supplies the loopback resource gate instead of remaining diagnostic only.
A functional or resource failure still fails its corresponding gate.

## Current Blocking Evidence

The corrected controlled comparison remains failed with the selected-base
channel monitor and notification boundary. Each run now calculates one p95
from all delivery observations in its five cost epochs. The stable
rendezvous-channel p95 medians are 92,245 ns for production and 151,618 ns for
the candidate. The stable bounded-channel medians are 62,039 ns and 119,819 ns.
The candidate ratios are approximately 164% and 193%, so both exceed the fixed
125% ceiling. The complete noise rule is also inconclusive. Production run
duration, production 16,384-task creation time, and production loopback p95
each have a range greater than 20% of their median.

The same artifact passes the allocation, common-population, throughput,
loopback latency, loopback resource, reclamation, CPU-saturation cancellation,
and identity-sensitive functional gates. Separate JVM CI checks pass the
one-carrier and two-carrier progress schedules and the production task-lifetime
and host-integration cases. These passing results do not override a failed
required gate. Production adoption and required-CI artifact validation remain
blocked until one complete recollection passes every artifact gate without
changing the workload, budgets, or retained wait algorithms merely to obtain
success.

## Completion

When one corrected controlled comparison passes all gates, switch the production
constructor to the virtual-thread candidate, require the passing artifact in CI,
and update the current execution specification. Then remove this proposal and
its Ready catalog entry; current specifications, executable examples, backend
tests, and the checked benchmark become the behavior authorities.
