---
role: proposal
update-when: The proposed lightweight task execution model, JVM baseline, task lifetime, cancellation, handler propagation, scheduling guarantees, or acceptance criteria change.
---

# Lightweight Tasks

Execute many concurrent Veln tasks without assigning a dedicated OS thread to
each task. The JVM implementation uses one Java virtual thread per task and
retains ordinary synchronous Veln function calls.

The [design decisions](#design-decisions) fix the remaining scope and its
acceptance gates. The implemented Java 25 prerequisite is owned by the
[execution](../specification/execution.md#runtime-readiness-and-host-boundaries),
[run](../specification/command-run.md#analysis-and-execution-gates), and
[test](../specification/command-test.md#gates-and-failures) specifications.
The virtual-thread worker and D2-D5 lifetime behavior are implemented, but D6
adoption remains incomplete until a corrected controlled comparison satisfies
every functional, allocation, resource, latency, noise, reclamation, and
lifecycle gate.

## Current Boundary And Motivation

The current [task contract](../specification/effects.md#concurrency-calls)
provides `task::spawn`, `task::spawn_with`, `task::join`, `task::cancel`, and
classified join errors. Arguments and returned values are frozen at task
boundaries. The [effect contract](../specification/effects.md#effect-labels)
defines handler propagation and lexical restoration. The
[execution contract](../specification/execution.md#runtime-readiness-and-host-boundaries)
requires task cleanup to finish before join reports cancellation.

The JVM implementation wraps each job in a `FutureTask` and starts one
unpooled virtual thread. The controlled platform-thread variant remains the
comparison baseline. This proposal now owns the incomplete adoption evidence;
it does not treat the implemented constructor or functional coverage as proof
that every D6 cost gate passes.

| Current implementation boundary | Consequence for the proposal |
| --- | --- |
| Task creation uses one virtual thread per task | Retain the controlled platform-thread variant as the adoption baseline. |
| Cancellation uses task-local state and an interrupt claimed once | Preserve delivery and cleanup ordering across the new worker type. |
| Handler lists use `InheritableThreadLocal`; cancellation state uses `ThreadLocal` | Keep state associated with the logical task while its carrier changes. |
| Channel send and receive use monitors and `Object.wait()` | Evaluate the selected JDK's monitor suspension behavior. |
| Channel select polls receivers and sleeps for 1 ms | Task allocation changes do not eliminate polling CPU or wakeup costs. |
| Socket readiness allocates a Selector for a waiting operation | Virtual threads do not remove selector, descriptor, or connection costs. |
| DNS resolution uses a bounded platform-worker pool | Keep host-operation limits distinct from the task execution model. |

These are implementation observations, not additional language guarantees.
The relevant files are listed in [References](#references).

## Proposed Scope And Execution Model

Keep the existing task source API and effect rows. A spawned job executes
ordinary Veln code, including nested calls, handlers, and cleanup regions.
Applications do not add async syntax or manually split a function at each wait.

Use one virtual thread for each task on the JVM. Let the JDK schedule virtual
threads over carrier platform threads. A carrier is an OS-backed thread that
temporarily executes a virtual thread. Task identity, handler state, and
cancellation state must not depend on carrier identity.

The JDK 25 [Thread API](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/Thread.html)
supports virtual threads, thread-local state, and inherited thread-local
values. Its virtual threads remain logical threads even when their carrier
changes. This fits Veln's existing synchronous JVM execution. Keep the task
handle and result protocol; a Java `Thread` remains a backend detail.

Virtual threads became final in Java 21,
but [JEP 491](https://openjdk.org/jeps/491), delivered in Java 24, also permits
virtual threads to unmount during monitor acquisition and `Object.wait()`.
That change matters directly to Veln's current channel implementation. A
Java 25 baseline avoids requiring a channel-lock rewrite solely to obtain
that suspension behavior. Native calls and some class-loading or initialization
paths can still occupy carriers; the proposal makes no universal nonblocking
host-call guarantee.

Do not pool virtual threads. The Java 25
[virtual-thread guide](https://docs.oracle.com/en/java/javase/25/core/virtual-threads.html)
recommends representing each concurrent task by a virtual thread and limiting
scarce external resources separately. Veln must retain bounded channel
backpressure and host resource limits. Cheap task creation does not imply
unlimited heap, descriptors, connections, or resolver capacity.

The initial proposal does not add structured concurrency, task groups,
automatic sibling cancellation, a public scheduler API, or another backend.
An initial implementation may retain the current channel and socket waiting
algorithms. D4 and D6 define the acceptance gates; failure of a gate blocks
adoption rather than expanding this implementation into a waiting-algorithm
rewrite.

## Observable Contracts

The following requirements apply together with the lifetime and fairness
decisions below.

| Boundary | Proposed observable contract |
| --- | --- |
| Spawn and context | Existing pure and effectful jobs remain callable with `spawn` or one context value through `spawn_with`. Existing freezing rules apply in both directions. |
| Waiting and progress | With a small carrier budget, tasks waiting on supported channel, timer, and join boundaries do not require one OS thread per task. Runnable peers can satisfy those waits when the program has no dependency cycle. |
| Join success and failure | Preserve the current distinction between successful values, classified join errors, and runtime failures that cross join as runtime failures. |
| Cancellation | Preserve cooperative delivery, the single atomic cancellation-initiation claim, the no-op for an already completed task, and the current completion race behavior. |
| Cleanup | Join does not report cancellation until registered cleanup has finished. Concurrent cancel calls do not repeatedly interrupt cleanup. Existing cleanup-failure precedence and related-failure ordering remain applicable. |
| Handler propagation | A child inherits the active handler stack at creation. Its lexical stack changes do not replace a sibling's stack. Shared provider resources retain their creating owner. |
| Task isolation | A carrier reused by another task does not transfer cancellation state or active handler frames between unrelated tasks. |
| Host effects | Without an injected handler, operations use real host effects. Deterministic substitutions remain scoped Veln handlers in separate test support. |
| Resource reclamation | Completed task bookkeeping and handler references become reclaimable when the application releases their handles; runtime lifetime tracking must not retain every completed task. |

Cancellation checkpoints and scheduler suspension are different mechanisms.
The current backend emits a cancellation checkpoint at function entry,
including its eligible tail-recursion loop entry. Checking cancellation does
not by itself yield execution to another virtual thread. No bounded CPU-task
fairness or cancellation latency is established by that checkpoint alone.

The current cancellation and cleanup behavior is supported by
[Veln PR #1706](https://github.com/oakcask/veln/pull/1706). That change joined
after cleanup and added a scoped `TaskJoin` observation hook. Preserve these
outcomes when changing the worker implementation; the historical PR is
rationale, while the current specifications remain authoritative.

## Alternatives And Related Work

| Alternative and inspected source | Finding and applicability | Design consequence |
| --- | --- | --- |
| Keep platform threads | The controlled baseline creates one platform worker for each task, including waiting tasks. | Retains the resource constraint this proposal addresses. Keep it as the measurement baseline. |
| Fixed platform-thread pool | With the current blocking call model, every worker can wait for queued children or channel peers that have no worker available. This is an inference from Veln's implementation, not a measured failure of a prototype. | A pool alone does not satisfy the progress requirement. Any pool-based alternative needs suspension or another demonstrated progress mechanism. |
| Java virtual threads: [JEP 444](https://openjdk.org/jeps/444), [JEP 491](https://openjdk.org/jeps/491), and the JDK 25 guide linked above | Preserve sequential Java stacks while multiplexing waiting tasks; monitor behavior depends on the JDK release. Virtual threads are daemon threads. | Selected JVM mechanism. The current Java baseline and D2 task lifetime prevent the change from being inferred from a constructor substitution alone. |
| Go runtime: [runtime developer documentation at Go 1.24.0](https://github.com/golang/go/blob/go1.24.0/src/runtime/HACKING.md) and [Go 1.14 runtime release notes](https://go.dev/doc/go1.14#runtime) | Go manages goroutines, OS threads, execution resources, and growing stacks; it also supports asynchronous preemption, including loops without function calls. Those mechanisms belong to Go's runtime and compiler. | Adopt the goal of multiplexing lightweight tasks. Do not promise Go-equivalent CPU scheduling or copy its stack machinery into ordinary JVM calls. D3 fixes the initial fairness boundary. |
| Async state machines: [Tokio 1.54.1 task documentation](https://docs.rs/tokio/1.54.1/tokio/task/) | Tasks schedule cooperatively and blocking work needs special treatment. Started `spawn_blocking` jobs generally cannot be aborted. This is a different execution and cancellation model. | Keep as an alternative for a future backend. It would require designing suspension across Veln calls, handlers, cleanup, and host operations; adding a Rust executor alone does not transform JVM execution. |

[Veln PR #570](https://github.com/oakcask/veln/pull/570) replaced numbered
public spawn APIs with one context argument. Lightweight execution does not
justify reintroducing arity-specific APIs or changing that source design.

## Acceptance And Verification Plan

The cases below remain the acceptance map. Focused evidence supports the
implemented D2-D5 behavior, but proposal completion still requires every L1-L8
row and every corrected D6 gate to pass together.

The checked controlled comparison reproduces a failed adoption verdict. Its
rendezvous-channel and bounded-channel delivery-latency medians exceed the D6
ceiling, and its complete metric set is inconclusive under the D6 noise rule.
These failures block adoption; they are not permission to change the fixed
workload, budgets, or retained wait algorithms.

| Case | Input or schedule | Required observation and planned evidence |
| --- | --- | --- |
| L1: allocation | Increasing populations of tasks held at a channel or timer wait | JVM integration harness identifies virtual workers and measures OS threads externally; worker allocation is not proportional to waiting task count. Use the controlled benchmark below for memory and cost. |
| L2: progress | A JVM carrier budget of one and two; parents spawn and join children; channel peers rendezvous or release a full buffer | The listed finite acyclic schedules, with bounded work between waits, complete within the harness timeout. A JVM integration test detects worker starvation independently of API return-value coverage. |
| L3: task boundary | Pure, effectful, and record-context jobs returning values or failures | Existing freezing, effect-row inference, callback, and failure-classification observations remain valid. Use compiler/backend evidence for compiler-known task boundaries and stdlib companion tests for source-defined wrapper behavior. |
| L4: cancellation | Cancel during channel, timer, join, and real socket waits; release a gated cleanup only after concurrent cancellation attempts finish | Join remains incomplete until cleanup finishes; one cancellation initiation interrupts the job; cleanup ordering and failure classification match the current contract. Use scoped Veln support plus JVM interruption and real-host integration tests. |
| L5: handler state | Nested handlers, children, siblings, unrelated tasks, and scope exit with retained resource aliases | Inheritance and lexical restoration remain correct; no carrier-state leakage or provider substitution occurs. Use Veln support and the backend handler/owner integration harness. |
| L6: entry lifetime | Normal and failed entry exit with unfinished children, nested spawns, and blocked cleanup | D2's transition table determines completion, output, exit, and isolation. Add CLI and reusable-test-entry integration evidence, including a child that spawns while draining and a rejected spawn while cancelling. |
| L7: CPU scheduling | More continuously runnable CPU tasks than carriers, plus a short task and a cancellation request | D3 promises no bounded CPU fairness. Record starvation and latency as diagnostics; use externally bounded runs rather than require the saturated workload to finish. |
| L8: failure and reclamation | Task start failure and repeated spawn/complete/release batches | D5 requires an abrupt startup failure, rollback of registration, and no published handle. Backend failure injection verifies rollback; retained-heap measurements check released handles and owners. |

Apply the [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy).
Returned-value permutations for source-defined standard-library APIs belong
in companion tests under `crates/veln-stdlib/veln/`. Compiler-known task
execution, carrier progress, JVM interruption, host resource ownership, and
process lifetime are independently identified integration contracts. A stdout
wrapper around an API assertion is not sufficient reason for a CLI case.

Deterministic schedules use scoped Veln handlers and explicit synchronization
in separate support. Real host tests establish behavior that a fake cannot,
including socket interruption and OS-thread allocation. Scheduler properties
may constrain a JVM integration workload; they must not select fake production
handlers. Every stress workload has process-tree time and memory bounds.

### Controlled Performance Comparison

Compare a prebuilt platform-thread baseline and a prebuilt candidate using the
same JDK, host, JVM flags, heap limit, and source workload. Build and analysis
time are outside the runtime measurements. Verify identical functional
outcomes before comparing cost.

| Workload | Measurements |
| --- | --- |
| Parked task populations at increasing sizes | OS thread count, RSS, heap, creation/completion time, and allocation failure point. |
| Rendezvous and bounded-channel pipelines | Throughput, CPU time, and end-to-end latency distribution. |
| Idle selects with occasional messages | Idle CPU, wakeups, and delivery latency; isolate the current polling cost. |
| Loopback connection workloads | Throughput, latency, OS threads, descriptors, and selector counts. |
| CPU tasks mixed with short jobs | Throughput and short-job latency under carrier saturation. |
| Repeated completed-task batches | Retained heap after collection with application handles released. |

Use warm-up, repeated independent JVM runs, alternating baseline/candidate
order, and reported dispersion. Record the host and runtime configuration
with the measurement artifact. Distinguish OS thread count from Java virtual
thread count; include JFR pinning events when diagnosing carrier occupancy.
Do not fail ordinary CI on wall-clock ratios from uncontrolled runners.
Use the populations and adoption budgets in D6. These are selected acceptance
thresholds, not measurements or claims about achieved capacity.

## Design Decisions

| Decision | Initial contract and rationale |
| --- | --- |
| D2: entry lifetime | Each run and each test invocation owns a task lifetime. Normal entry completion drains all admitted tasks; failed entry completion cancels and drains them. The entry's non-daemon launcher remains alive until draining finishes. A subsequent test cannot inherit children or handlers from the preceding test. See the transition table. |
| D3: CPU fairness | Guarantee peer progress only at supported carrier-releasing waits. Do not add yield insertion or promise CPU time slicing, bounded cancellation latency, or starvation freedom under CPU saturation. Existing cancellation checkpoints remain. The JDK guide's scheduling model does not supply a Veln fairness bound; stronger fairness requires separate compiler and scheduler work. |
| D4: waits | Retain channel monitors, polling select, per-operation socket selectors, and bounded DNS workers. Supported carrier-progress acceptance covers channel, timer, and join waits; loopback sockets have separate host-integration coverage. Native calls, filesystem calls, DNS, and class initialization have no carrier-release guarantee. Select idle cost and descriptor counts remain measured diagnostics. No new wakeup protocol or tie rule is introduced. |
| D5: admission and ownership | No public task-count limit or queue. Admission is subject to host resources and lifetime state. Allocation or worker-start failure raises a runtime failure at spawn, preserves its cause, publishes no handle, and removes any partial lifetime registration. A closed lifetime rejects spawn as a runtime failure before executing the job. The lifetime owns only incomplete jobs; completion removes registration even if no join occurs. Handles retain their own result until released. No permanent completed-task history is permitted. |
| D6: adoption budgets | Use the controlled protocol below. Functional, lifecycle, allocation, and retained-state gates are required. Throughput and latency budgets apply on the controlled comparison runner only; uncontrolled CI does not gate ratios. Passing functional tests alone does not establish adoption readiness. |

### Entry Lifetime Transitions

An admitted task belongs to exactly one invocation, including descendants.
Registration and the transition to an empty, closed lifetime must be atomic
with respect to spawn. Entry lexical cleanup finishes before entry completion
is classified; draining does not extend a lexical handler owner's lifetime.

| State | Event or guard | Next state | Observable result |
| --- | --- | --- | --- |
| Running | Entry and its lexical cleanup succeed | Draining | Preserve the pending entry value; wait for all admitted jobs, including their cleanup. |
| Running or Draining | An admitted task spawns a child | Same state | Admit the descendant into the same invocation before returning its handle. During Draining only still-admitted tasks can spawn. |
| Running | Entry fails, returns an entry error, or is cancelled | Cancelling | Close admission, request cancellation once on every incomplete task, and wait for cleanup. Include tasks admitted before the transition. |
| Cancelling | Any spawn attempt | Cancelling | Fail at spawn; no child starts. A spawn failure inside cleanup follows existing cleanup-failure precedence. |
| Draining | Last admitted task completes | Closed | Publish the pending entry result and allow process exit or the next test. |
| Cancelling | Last admitted task completes | Closed | Publish the original entry failure; append child cleanup failures as related context in task admission order, preserving each task's internal cleanup order. |
| Any live state | A child completes with a value or ordinary task failure | Same state, or Closed if this empties a draining lifetime | Remove its registration. Join retains existing results and failure propagation. An unjoined child failure does not independently change the entry result. |
| Draining or Cancelling | A job or cleanup never finishes | Same state | The invocation remains incomplete. There is no implicit deadline, forced termination, or truncated committed output. |
| Closed | A retained handle is joined; another invocation starts | Closed for the old lifetime | Its settled join result remains available; a new invocation creates a distinct lifetime. |

Already committed output remains visible in all states. Relative ordering of
output from concurrent tasks is unspecified. An entry failure stays primary;
cancellation requested only to end its children does not replace it. Ordinary
child failures still propagate only through join. This preserves the task
handle protocol rather than introducing task-group failure propagation.

### Controlled Adoption Budgets

These values are project acceptance choices. They are not universal capacity
or latency guarantees. The measurement artifact must record both successes
and resource-limit failures; an uncompleted baseline population is not a
successful comparison point.

| Concern | Workload and gate |
| --- | --- |
| Bounded runner | Prebuilt programs; 4 GiB process-tree memory ceiling, 512 MiB Java heap, and 180 s per JVM run. Stop a population ladder after its first resource failure. CI progress cases use at most 1,024 jobs, a 2 GiB process-tree limit, and 30 s per case. |
| Samples and noise | One discarded warm-up run and seven independent measured JVM runs per configuration, alternating baseline and candidate. Report medians, ranges, and per-run latency p95; compare medians of those p95 values. A timeout or OOM is a failed sample, never omitted. If a metric's range exceeds 20% of its median, the comparison is inconclusive and blocks adoption until a stable run is obtained. |
| Waiting-task allocation | Park 256, 1,024, 4,096, then 16,384 jobs. With carrier parallelism and maximum pool size both fixed to two, candidate OS-thread growth between 256 and 16,384 parked jobs is at most eight. The candidate must finish the 16,384-job run within the runner bounds. This is a benchmark gate, not an API task quota. |
| Common-population cost | At 1,024 jobs, both implementations must complete. Candidate median peak RSS must not exceed baseline; its worker OS-thread count must be at most one quarter of baseline. Measure total process threads separately. |
| Runtime regressions | At the common population, channel-pipeline and loopback throughput must be at least 80% of baseline, and delivery-latency p95 at most 125% of baseline under the same offered load. Publish source workload, payload sizes, message/connection counts, and offered rate in the benchmark artifact before collecting either result; changes invalidate the pair. |
| Reclamation | Ten batches of 1,024 spawn/complete/release jobs; after each batch release application handles and collect before recording live heap. The last five samples must remain within a 10% band of their median; backend lifecycle checks must also observe zero completed registrations and no retained released owners. Heap flatness alone is insufficient. |
| Diagnostic metrics | Polling-select idle CPU and wakeups, descriptor/selector counts, CPU-saturation latency, and JFR pinning events are reported without an adoption ratio. A resource leak or functional failure still fails the corresponding correctness gate. |

## Implementation And Specification Route

The remaining design questions are resolved by D2-D6. The implementation keeps
the selected lifetime model and JVM worker change together; a worker-constructor
substitution alone would not resolve daemon shutdown behavior.

Keep the performance comparison separate from any channel/select algorithm
change so that the contribution of each change can be assessed. Reconsider the
preferred mechanism if required CPU fairness, host-call behavior, another
backend, or measured resource costs invalidate its assumptions.

The current effects, execution, run, and test specifications own the implemented
D2-D5 behavior. Keep D6 acceptance evidence proposal-owned until every gate
passes. Remove the completed proposal and catalog entry only after the complete
selected scope satisfies that evidence.

## References

- Current task and channel implementation:
  [`concurrency.java.inc`](../../crates/veln-backend-jvm/src/runtime/concurrency.java.inc).
- Handler state:
  [`values.java.inc`](../../crates/veln-backend-jvm/src/runtime/values.java.inc).
- Socket readiness and bounded DNS workers:
  [`effects.java.inc`](../../crates/veln-backend-jvm/src/runtime/effects.java.inc).
- Cancellation checkpoint generation:
  [`function_flow.rs`](../../crates/veln-backend-jvm/src/classfile/function_flow.rs).
- Runtime compilation and CI baseline:
  [`build.rs`](../../crates/veln-backend-jvm/build.rs) and
  [`test--jvm-backend.yaml`](../../.github/workflows/test--jvm-backend.yaml).
- Existing cancellation ordering and failure-identity evidence:
  [`JoinBlockingHarness.java`](../../crates/veln-backend-jvm/test-support/JoinBlockingHarness.java)
  and [`CancellationIdentityHarness.java`](../../crates/veln-backend-jvm/test-support/CancellationIdentityHarness.java).
