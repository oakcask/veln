---
role: proposal
update-when: The filesystem IO F2–F5 feasibility scope, provider adapter prototype, acceptance evidence, or completion and promotion conditions change.
---

# Filesystem IO Feasibility

Implement a bounded source/adapter prototype that tests whether the proposed
filesystem and shared byte IO design can preserve capabilities, provider
identity, state, progress, and cleanup through ordinary Veln handlers.
This Ready target owns F2–F5. Ready authorizes building the prototype, not
adopting the production API.

## Scope And Dependency Direction

This proposal owns the remaining feasibility gates F2–F5 extracted from
[Filesystem And Shared Byte IO](filesystem-io.md). That proposal remains in
Design In Progress until this work completes. This work does not depend on
its production implementation or promotion to Ready.

The following sections of the filesystem proposal define the prototype's
design inputs. The static source boundary, compiler-known adapter names and
opaque identities, and bounded five-family registration and projection are
implemented and specified by
[Standard-library filesystem and byte IO](../specification/standard-library-filesystem-io.md).
The table lists only the remaining proposed obligations:

| Design input | Prototype obligation |
| --- | --- |
| [Provider-bound dispatch prerequisite](filesystem-io.md#provider-bound-dispatch-prerequisite) | Invoke captured handlers through their registered resource, and add atomic `State<T>` operations. |
| [Outcomes and error translation](filesystem-io.md#outcomes-and-error-translation) | Preserve bytes, known and unknown progress, error context, and no-progress behavior. |
| [Namespace, seek, and mutation transitions](filesystem-io.md#namespace-seek-and-mutation-transitions) | Observe successful and failed operations through bytes, cursors, bindings, and commit facts. |
| [Concurrent operations and cleanup](filesystem-io.md#concurrent-operations-and-cleanup) | Preserve per-open admission, release obligations, aliases, and failure precedence. |

Implement only the adapter and source/provider support needed to demonstrate
these contracts. F4 retains every outcome and transition row in the linked
contracts; a small prototype is not permission to omit difficult failures.
Scripted failures belong in separate scoped Veln test support. Focused host
adapters establish host-specific behavior that a fake cannot prove.

Production API rollout, migration of existing filesystem callers, removal of
old compiler-known calls, and the complete network-to-byte adapter rollout
remain in the filesystem proposal. This target does not introduce buffering,
confinement, new effect syntax, or performance guarantees.

## Atomic State Cancellation Boundary

The atomic state prerequisite follows the current cooperative task-cancellation
model. Cancellation must not publish a partial transition. It also must not be
deferred across every generated cancellation checkpoint in an admitted
callback. The callback and publication boundary has these planned outcomes:

| State and event | Required outcome |
| --- | --- |
| Waiting for admission; cancellation is observed | Do not enter the callback. Preserve the old state, release any admission bookkeeping, and report task cancellation. |
| Callback admitted; cancellation is observed at a generated callback checkpoint before the callback returns | Abort the callback. Preserve the old state, release the cell, and report task cancellation. A compiled tail-recursive callback must reach this outcome instead of retaining the cell indefinitely. |
| Callback returns before cancellation is observed | Publish the replacement and result as one transition. Release the cell before reporting task cancellation. Do not return the callback result to the cancelled task. |
| Callback or runtime fails before publication | Preserve the old state, release the cell, and propagate the failure. |

Cancellation is cooperative. A callback that neither returns, fails, nor
reaches a generated cancellation checkpoint has no bounded cancellation
latency and may retain the cell. This is a limit of the task cancellation
model, not permission to suppress checkpoints that the compiler generated.
The prototype does not add a wall-clock timeout, execution fuel, or termination
analysis to make arbitrary pure callbacks preemptible.

## Acceptance Gates

Expected bytes and state come from the linked contracts and independently
specified inputs, not copied implementation output. Each gate requires both
its positive observations and its failure or preserved-state observations.

| Gate | Required observation | Evidence and status |
| --- | --- | --- |
| F2: Veln-only allocation | Two fake filesystem instances return opaque resources using Veln handler state without any host fs/net call. Nested shadowing does not redirect an older handle; cross-provider copy transfers the selected bytes. | Companion tests with distinct provider contents and operation logs; a backend dispatch case checks captured provider identity and absence of host fallback. |
| F3: state and publication | Concurrent children cannot lose completed writes. Failure and cancellation preserve the transition boundary above. Closing the owner rejects late publication and releases the late host resource. | Scoped Veln support plus backend atomic-state and owner-lifecycle cases. Force cancellation before entry, at a generated checkpoint after admission, and after a callback has returned but before its result is observed. Runtime callback failure preserves old state and releases the cell. Gate a host open across owner closure and observe release instead of publication, including retained cleanup failure. |
| F4: transfer and mutation | Every outcome and transition row in the linked outcome and namespace contracts has independent expected bytes, cursor, commit, and namespace observations, including unsupported rename and unknown host progress. | Shared companion conformance cases for fake and supported system behavior, with scripted failures and progress logs; focused real-host cases for native translation, unsupported atomic operations, and uncertain host outcomes. |
| F5: cleanup | Close failure facts, retry, reverse cleanup, primary/related failures, and owner exit with retained child aliases obey the linked cleanup table. | Companion tests for `No`, `Yes`, and `Unknown` release facts and retry; backend owner-exit and cancellation cases; human and JSON projections for primary failure and ordered related cleanup failures. |

F3 must demonstrate cancellation atomicity of `update` before the fake relies
on that operation. The admitted-checkpoint case must run a compiled Veln
callback, including a tail-recursive path that continues to cross generated
checkpoints until cancellation is delivered. Tests must control entry,
checkpoint observation, commit, publication, and cleanup boundaries explicitly
rather than relying on timing or an external filesystem to produce a rare
failure.

If F2–F3 cannot be demonstrated with the named adapter extension, revise the
resource design and affected acceptance contracts before continuing to F4–F5.
A platform-only fake or an assumed unimplemented state/dispatch primitive
does not satisfy a gate. Such a failure does not promote the parent proposal.

## Implementation Sequence And Evidence Placement

1. Demonstrate the atomic state boundary required by F3. Add separate Veln
   provider support and use it to demonstrate F2 and the remaining publication
   observations in F3. Keep fake state and failure plans in Veln.
2. Apply the outcome and transition tables to F4 and F5. Add host-boundary
   cases only for observations that require a real host or backend integration.

| Evidence | Placement and local verification route |
| --- | --- |
| Standard-library API results, progress, state, operation ordering, and scripted failure behavior | Companion `.test.veln` files under `crates/veln-stdlib/veln/`, using separate Veln test support; run `bash scripts/agent-stdlib-test`. |
| Compiler capability, effect rejection, and generic state lowering | Focused compiler tests or checked cases under `examples/specification/check/`; successful lowering must retain the inferred state type for string, record, and list or constructor initial values in checked core and typed IR; run the selected cases through `bash scripts/agent-test`. |
| Captured dispatch, task freezing, atomic state, publication, and release | Focused `veln-backend-jvm` tests through `bash scripts/agent-test -p veln-backend-jvm`; identify the backend invariant each case checks. |
| Real-host semantics and human/JSON cleanup projections | Focused host integration or CLI cases under the [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy); run the selected integration tests through `bash scripts/agent-test`. |

When implementing handler dispatch and lifetime support, follow the
effect-handler maintenance workflow, including the `effect_boundary_policy`
and `effect_injection` checks. Use the guarded runners for execution bounds.
No compiler or CLI wrapper substitutes for companion coverage of API behavior.

## Related Work And Design Basis

The current [nominal effect boundary](../specification/effects.md#effect-labels)
provides source-defined operations and handlers. It does not establish the
proposed general resource allocation or captured-dispatch API. F2 therefore
checks the explicit adapter extension rather than treating nominal
dispatch alone as proof of feasibility. Extending ordinary `perform` to route
all operations by resource identity remains outside the selected design.

The current [task boundary](../specification/effects.md#concurrency-calls)
defines cancellation as cooperative. The atomic state transition uses that
existing boundary: a generated checkpoint may cancel an admitted callback
before publication, while a completed publication remains committed. Deferring
all callback checkpoints was rejected because a compiled tail-recursive
callback could retain the cell after cancellation. Unconditional preemption
was rejected because the runtime does not provide termination analysis or a
safe arbitrary-thread stop.

The current [JVM execution boundary](../specification/execution.md#runtime-readiness-and-host-boundaries)
ties system network resources to handler owners, shares those owners with
child tasks, and rejects publication after owner closure. This is relevant
local precedent for F3 and F5. It does not provide Veln-only allocation or
atomic Veln state. The prototype must demonstrate those separately; reusing
a host network handle as the fake filesystem would not establish them.

The current [cleanup rules and limits](../specification/execution.md#cleanup-limits)
exclude implicit close and process-termination guarantees. The execution
boundary also defines primary and related network cleanup failures. F5
tests the proposed owner extension against that failure model rather than
assuming that a deferred close can discard its outcome. The external API
comparisons in the [filesystem design](filesystem-io.md#design-decisions)
remain rationale for its IO contracts; they do not prove Veln adapter support.

## Completion And Parent Promotion

This proposal completes only when F2–F5 have checked evidence at the stated
boundaries, with every linked outcome and transition row accounted for.
Passing F2–F3 alone is insufficient. A failed gate keeps the target incomplete
until the design and its independent expectations are reconciled.

For any adapter or API behavior retained as implemented, update the smallest
current effects, execution, or standard-library specification alongside its
executable evidence. Do not describe prototype-only surfaces as available
production APIs. Keep the remaining production rollout in `filesystem-io.md`.

After all gates pass, review the parent proposal against that evidence before
moving it to Ready. Remove this completed proposal and its catalog entry,
replace the parent's dependency link with routes to the resulting current
specifications and checked evidence, and leave only unimplemented work in the
parent. Passing the gates does not itself complete the filesystem rollout.
