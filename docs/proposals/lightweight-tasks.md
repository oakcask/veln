---
role: proposal
update-when: The proposed lightweight task execution model, JVM baseline, task lifetime, cancellation, handler propagation, scheduling guarantees, or acceptance criteria change.
---

# Lightweight Tasks

Execute many concurrent Veln tasks without assigning a dedicated OS thread to
each task. The proposed JVM implementation uses one Java virtual thread per
task and retains ordinary synchronous Veln function calls.

This proposal establishes a design direction. It is not ready for
implementation selection until the [Open Questions](#open-questions) have
concrete answers for the initial scope. All acceptance evidence below is
planned. No throughput improvement, task capacity, or memory reduction has
been measured for Veln.

## Current Boundary And Motivation

The current [task contract](../specification/effects.md#concurrency-calls)
provides `task::spawn`, `task::spawn_with`, `task::join`, `task::cancel`, and
classified join errors. Arguments and returned values are frozen at task
boundaries. The [effect contract](../specification/effects.md#effect-labels)
defines handler propagation and lexical restoration. The
[execution contract](../specification/execution.md#runtime-readiness-and-host-boundaries)
requires task cleanup to finish before join reports cancellation.

The JVM implementation wraps each job in a `FutureTask` and starts a new
platform `Thread`. A task waiting on a channel, timer, or another task thus
still owns an OS thread. This limits useful concurrency for workloads with
many waiting tasks, including connection services and fan-out computations.
The proposal targets that allocation model; it does not assume that every
application becomes faster.

| Current implementation boundary | Consequence for the proposal |
| --- | --- |
| Task creation uses `new Thread(future)` | Replace dedicated platform workers with virtual threads. |
| Cancellation uses task-local state and an interrupt claimed once | Preserve delivery and cleanup ordering across the new worker type. |
| Handler lists use `InheritableThreadLocal`; cancellation state uses `ThreadLocal` | Keep state associated with the logical task while its carrier changes. |
| Channel send and receive use monitors and `Object.wait()` | Evaluate the selected JDK's monitor suspension behavior. |
| Channel select polls receivers and sleeps for 1 ms | Task allocation changes do not eliminate polling CPU or wakeup costs. |
| Socket readiness allocates a Selector for a waiting operation | Virtual threads do not remove selector, descriptor, or connection costs. |
| DNS resolution uses a bounded platform-worker pool | Keep host-operation limits distinct from the task execution model. |
| Runtime compilation uses `javac --release 8`; JVM CI uses Java 25 | Select an explicit supported build and execution baseline. |

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

The preferred baseline is Java 25 for building and executing the runtime.
This remains a decision under Q1. Virtual threads became final in Java 21,
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
algorithms if the acceptance evidence supports the selected scope. Q4 decides
whether their costs require an accompanying change.

## Observable Contracts

The following requirements apply to the proposed execution model. They do not
resolve the lifetime and fairness choices in Open Questions.

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
| Keep platform threads | Existing runtime creates one platform worker for each task, including waiting tasks. | Retains the resource constraint this proposal addresses. Keep it as the measurement baseline. |
| Fixed platform-thread pool | With the current blocking call model, every worker can wait for queued children or channel peers that have no worker available. This is an inference from Veln's implementation, not a measured failure of a prototype. | A pool alone does not satisfy the progress requirement. Any pool-based alternative needs suspension or another demonstrated progress mechanism. |
| Java virtual threads: [JEP 444](https://openjdk.org/jeps/444), [JEP 491](https://openjdk.org/jeps/491), and the JDK 25 guide linked above | Preserve sequential Java stacks while multiplexing waiting tasks; monitor behavior depends on the JDK release. Virtual threads are daemon threads. | Preferred JVM mechanism. Decide the Java baseline and task lifetime explicitly; do not infer either from a constructor substitution. |
| Go runtime: [runtime developer documentation at Go 1.24.0](https://github.com/golang/go/blob/go1.24.0/src/runtime/HACKING.md) and [Go 1.14 runtime release notes](https://go.dev/doc/go1.14#runtime) | Go manages goroutines, OS threads, execution resources, and growing stacks; it also supports asynchronous preemption, including loops without function calls. Those mechanisms belong to Go's runtime and compiler. | Adopt the goal of multiplexing lightweight tasks. Do not promise Go-equivalent CPU scheduling or copy its stack machinery into ordinary JVM calls. Q3 owns fairness. |
| Async state machines: [Tokio 1.54.1 task documentation](https://docs.rs/tokio/1.54.1/tokio/task/) | Tasks schedule cooperatively and blocking work needs special treatment. Started `spawn_blocking` jobs generally cannot be aborted. This is a different execution and cancellation model. | Keep as an alternative for a future backend. It would require designing suspension across Veln calls, handlers, cleanup, and host operations; adding a Rust executor alone does not transform JVM execution. |

[Veln PR #570](https://github.com/oakcask/veln/pull/570) replaced numbered
public spawn APIs with one context argument. Lightweight execution does not
justify reintroducing arity-specific APIs or changing that source design.

## Acceptance And Verification Plan

These cases are requirements for future implementation evidence. They are not
new tests in this proposal change and are not reported as passing.

| Case | Input or schedule | Required observation and planned evidence |
| --- | --- | --- |
| L1: allocation | Increasing populations of tasks held at a channel or timer wait | JVM integration harness identifies virtual workers and measures OS threads externally; worker allocation is not proportional to waiting task count. Use the controlled benchmark below for memory and cost. |
| L2: progress | A JVM carrier budget of one and two; parents spawn and join children; channel peers rendezvous or release a full buffer | All finite acyclic schedules complete within the harness timeout. A JVM integration test detects worker starvation independently of API return-value coverage. |
| L3: task boundary | Pure, effectful, and record-context jobs returning values or failures | Existing freezing, effect-row inference, callback, and failure-classification observations remain valid. Use compiler/backend evidence for compiler-known task boundaries and stdlib companion tests for source-defined wrapper behavior. |
| L4: cancellation | Cancel during channel, timer, join, and real socket waits; release a gated cleanup only after concurrent cancellation attempts finish | Join remains incomplete until cleanup finishes; one cancellation initiation interrupts the job; cleanup ordering and failure classification match the current contract. Use scoped Veln support plus JVM interruption and real-host integration tests. |
| L5: handler state | Nested handlers, children, siblings, unrelated tasks, and scope exit with retained resource aliases | Inheritance and lexical restoration remain correct; no carrier-state leakage or provider substitution occurs. Use Veln support and the backend handler/owner integration harness. |
| L6: entry lifetime | Normal and failed entry exit with unfinished children, nested spawns, and blocked cleanup | Exact completion, cancellation, output, and exit observations follow Q2's selected transition table. Add CLI integration evidence for process lifetime. |
| L7: CPU scheduling | More continuously runnable CPU tasks than carriers, plus a short task and a cancellation request | Record starvation and latency. Any fairness guarantee selected in Q3 requires a bounded, independently observable acceptance case. |
| L8: failure and reclamation | Task start failure and repeated spawn/complete/release batches | Q5 defines failure reporting and ownership. The selected implementation leaves no permanently registered completed tasks or inherited resource owners; verify with backend lifecycle evidence and retained-heap measurements. |

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
Population sizes, repetitions, regression budgets, and the minimum useful
improvement remain Q6; choose them before using results to accept the design.

## Open Questions

| Question | Options and required resolution before implementation selection |
| --- | --- |
| Q1: supported Java baseline | Prefer Java 25. Decide the minimum build JDK, runtime JDK, runtime classfile target, and unsupported-runtime diagnostic. If Java 21 or an older fallback is retained, specify its separate behavior and verification burden; CI's current Java version does not establish support policy. |
| Q2: task lifetime at entry exit | Choose wait-for-all, cancel-and-wait, or process exit without waiting. Virtual threads are always daemon, while ordinary current tasks inherit a non-daemon parent. Define normal exit, failed entry, child failure, nested spawn during shutdown, new-spawn admission, blocked cleanup, and whether lifetimes are per run, per test, or shared across a test process. Write a transition table covering outputs, exit result, and failure precedence. |
| Q3: CPU fairness and cancellation responsiveness | Decide whether the initial contract promises only progress when supported waits release carriers or also progress during CPU saturation. If stronger guarantees are required, choose a yield mechanism and insertion boundaries, and specify observable limits and overhead measurements. Cancellation checks alone are insufficient. |
| Q4: waiting mechanisms in the initial scope | Determine from controlled measurements whether polling select and per-operation selectors can remain. If either changes, specify wakeup registration, lost-wakeup prevention, close/cancel races, tie selection, and resource bounds before implementation. Native or filesystem waits need an explicit supported-progress boundary. |
| Q5: admission, startup failure, and ownership | Decide whether task admission is unbounded subject to host resources or has an explicit limit. Define failure when a worker cannot start, cleanup of partially registered lifetime state, and who releases runtime-owned task bookkeeping. The current infallible-looking spawn signature must not hide an unspecified failure policy. |
| Q6: measurable acceptance budgets | Select representative population sizes, runner limits, repetitions, noise treatment, and resource/latency budgets. Decide which metrics gate adoption and which remain diagnostic; do not invent a Veln speedup or a universal task capacity. |

## Implementation And Specification Route

After resolving the questions, move this proposal to Ready only when its
observable contracts and acceptance plan are complete. Implement the selected
lifetime model and JVM worker changes together; changing the worker constructor
alone does not resolve daemon shutdown behavior.

Keep the performance comparison separate from any channel/select algorithm
change so that the contribution of each change can be assessed. Reconsider the
preferred mechanism if required CPU fairness, host-call behavior, another
backend, or measured resource costs invalidate its assumptions.

Promote implemented task behavior to `effects.md` and `execution.md`, and any
new command lifetime or Java-requirement behavior to the matching command
specification. Update executable evidence at those boundaries. Remove the
completed proposal and catalog entry once the implemented specification owns
the complete selected scope.

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
