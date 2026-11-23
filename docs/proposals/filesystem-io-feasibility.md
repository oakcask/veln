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
| [Provider-bound dispatch prerequisite](filesystem-io.md#provider-bound-dispatch-prerequisite) | Invoke captured handlers through their registered resource, and use the implemented atomic `State<T>` operations for fake provider state. |
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

The atomic state prerequisite is current behavior specified by
[Standard-library filesystem and byte IO](../specification/standard-library-filesystem-io.md#file-view-runtime-prototype)
and [Execution](../specification/execution.md#runtime-readiness-and-host-boundaries).
It already has checked compiler, companion, and backend evidence for generic
state, shared identity, serialization, failure preservation, cooperative
cancellation, publication, reentrancy rejection, and release. This proposal
does not redefine that contract. F3 retains only the provider-fake and owner
publication observations that use it.

## Acceptance Gates

Expected bytes and state come from the linked contracts and independently
specified inputs, not copied implementation output. Each gate requires both
its positive observations and its failure or preserved-state observations.

| Gate | Required observation | Evidence and status |
| --- | --- | --- |
| F2: Veln-only allocation | Two fake filesystem instances return opaque resources using Veln handler state without any host fs/net call. Nested shadowing does not redirect an older handle; cross-provider copy transfers the selected bytes. | Companion tests with distinct provider contents and operation logs; a backend dispatch case checks captured provider identity and absence of host fallback. |
| F3: state and publication | Concurrent children cannot lose completed writes. Failure and cancellation preserve the linked atomic-state transition boundary. Closing the owner rejects late publication and releases the late host resource. | The atomic state prerequisite has separate companion, compiler, and backend evidence. The remaining gate needs provider-fake write observations and owner-lifecycle cases, including a host open gated across owner closure and retained cleanup failure. |
| F4: transfer and mutation | Every outcome and transition row in the linked outcome and namespace contracts has independent expected bytes, cursor, commit, and namespace observations, including unsupported rename and unknown host progress. | Shared companion conformance cases for fake and supported system behavior, with scripted failures and progress logs; focused real-host cases for native translation, unsupported atomic operations, and uncertain host outcomes. |
| F5: cleanup | Close failure facts, retry, reverse cleanup, primary/related failures, and owner exit with retained child aliases obey the linked cleanup table. | Companion tests for `No`, `Yes`, and `Unknown` release facts and retry; backend owner-exit and cancellation cases; human and JSON projections for primary failure and ordered related cleanup failures. |

The remaining F3 gate must use this implemented cancellation boundary when the
provider fake is added. It must still demonstrate concurrent fake writes,
owner closure, and late host-resource release. Those observations, together
with the implemented atomic prerequisite, must control entry, checkpoint
observation, commit, publication, and cleanup explicitly rather than relying
on timing or an external filesystem to produce a rare failure.

If F2–F3 cannot be demonstrated with the named adapter extension, revise the
resource design and affected acceptance contracts before continuing to F4–F5.
A platform-only fake or an assumed unimplemented dispatch primitive does not
satisfy a gate. Such a failure does not promote the parent proposal.

## Implementation Sequence And Evidence Placement

1. Use the implemented atomic state boundary to add separate Veln provider
   support and demonstrate F2 and the remaining publication observations in
   F3. Keep fake state and failure plans in Veln.
2. Apply the outcome and transition tables to F4 and F5. Add host-boundary
   cases only for observations that require a real host or backend integration.

| Evidence | Placement and local verification route |
| --- | --- |
| Standard-library API results, progress, state, operation ordering, and scripted failure behavior | Companion `.test.veln` files under `crates/veln-stdlib/veln/`, using separate Veln test support; run `bash scripts/agent-stdlib-test`. |
| Compiler capability and effect rejection for remaining provider calls | Focused compiler tests or checked cases under `examples/specification/check/`; run the selected cases through `bash scripts/agent-test`. |
| Captured dispatch, owner publication, and resource release | Focused `veln-backend-jvm` tests through `bash scripts/agent-test -p veln-backend-jvm`; identify the backend invariant each case checks. |
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
local precedent for F3 and F5. It does not provide Veln-only allocation or the
filesystem-provider owner lifecycle. The prototype must demonstrate those
separately; reusing a host network handle as the fake filesystem would not
establish them.

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
