---
role: proposal
update-when: The filesystem IO F4–F5 feasibility scope, provider adapter prototype, acceptance evidence, or completion and promotion conditions change.
---

# Filesystem IO Feasibility

Implement a bounded source/adapter prototype that tests whether the proposed
filesystem and shared byte IO design can preserve capabilities, provider
identity, state, progress, and cleanup through ordinary Veln handlers.
This Ready target owns F4–F5. Ready authorizes building the prototype, not
adopting the production API.

## Scope And Dependency Direction

This proposal owns the remaining feasibility gates F4–F5 extracted from
[Filesystem And Shared Byte IO](filesystem-io.md). That proposal remains in
Design In Progress until this work completes. This work does not depend on
its production implementation or promotion to Ready.

The implemented provider foundation is specified by
[Standard-library filesystem and byte IO](../specification/standard-library-filesystem-io.md).
It owns the completed F3 observations. Concurrent child callers through two
same-owner writer registrations retain both completed transitions in one
shared fake-provider state cell. Owner closure wins publication admission,
rejects and releases a late provider resource, retains a release failure,
disables facade and lexical use before cleanup, and grants only the matching
owner access to the exact releasing resource and its projected capabilities.
Ordinary recursive close and stale same-slot aliases remain rejected. Release
callbacks run outside registry admission so inherited child rejection can
progress. Facade and accepted lexical opens use one owner-matched publication
admission boundary, and one owner's active cleanup does not suppress another
owner's late-resource release. Cancellation first delivered by a release
handler remains the primary unwind after all selected resources receive their
bounded release attempt and registry removal. The current specification owns
the exact results, ordering, and preserved-state contracts.
The following sections of the filesystem proposal define the remaining
prototype obligations:

| Design input | Prototype obligation |
| --- | --- |
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

## Acceptance Gates

Expected bytes and state come from the linked contracts and independently
specified inputs, not copied implementation output. Each gate requires both
its positive observations and its failure or preserved-state observations.

| Gate | Required observation | Evidence and status |
| --- | --- | --- |
| F4: transfer and mutation | Every outcome and transition row in the linked outcome and namespace contracts has independent expected bytes, cursor, commit, and namespace observations, including unsupported rename and unknown host progress. | Shared companion conformance cases for fake and supported system behavior, with scripted failures and progress logs; focused real-host cases for native translation, unsupported atomic operations, and uncertain host outcomes. |
| F5: cleanup | Close failure facts, retry, reverse cleanup, primary/related failures, and owner exit with retained child aliases obey the linked cleanup table. Every cleanup attempt reuses the implemented F3 restricted boundary: the matching owner can inspect only the exact releasing resource and capabilities projected from it, while ordinary use, recursive close, stale same-slot aliases, and foreign owners are rejected. | Companion tests for `No`, `Yes`, and `Unknown` release facts and retry across the restricted boundary; backend owner-exit and cancellation cases; human and JSON projections for primary failure and ordered related cleanup failures. |

## Implementation Sequence And Evidence Placement

1. Apply the outcome and transition tables to F4 and F5. Keep fake state and
   failure plans in Veln. Add host-boundary cases only for observations that
   require a real host or backend integration.

| Evidence | Placement and local verification route |
| --- | --- |
| Standard-library API results, progress, state, operation ordering, and scripted failure behavior | Companion `.test.veln` files under `crates/veln-stdlib/veln/`, using separate Veln test support; run `bash scripts/agent-stdlib-test`. |
| Compiler capability and effect rejection for remaining provider calls | Focused compiler tests or checked cases under `examples/specification/check/`; run the selected cases through `bash scripts/agent-test`. |
| Real-host semantics and human/JSON cleanup projections | Focused host integration or CLI cases under the [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy); run the selected integration tests through `bash scripts/agent-test`. |

When implementing handler lifetime support, follow the
effect-handler maintenance workflow, including the `effect_boundary_policy`
and `effect_injection` checks. Use the guarded runners for execution bounds.
No compiler or CLI wrapper substitutes for companion coverage of API behavior.

## Related Work And Design Basis

The current [JVM execution boundary](../specification/execution.md#runtime-readiness-and-host-boundaries)
and [filesystem and byte IO specification](../specification/standard-library-filesystem-io.md#owner-closure-and-restricted-release)
own the implemented F3 provider lifecycle. Network owner cleanup remains local
precedent for the broader F5 failure model; a network handle does not substitute
for filesystem-provider outcome and transition evidence.

The current [cleanup rules and limits](../specification/execution.md#cleanup-limits)
exclude implicit close and process-termination guarantees. The execution
boundary also defines primary and related network cleanup failures. F5
tests the proposed owner extension against that failure model rather than
assuming that a deferred close can discard its outcome. The external API
comparisons in the [filesystem design](filesystem-io.md#design-decisions)
remain rationale for its IO contracts; they do not prove Veln adapter support.

## Completion And Parent Promotion

This proposal completes only when F4–F5 have checked evidence at the stated
boundaries, with every linked outcome and transition row accounted for. F3 is
implemented and specified independently. A failed remaining gate keeps the
target incomplete until the design and its independent expectations are
reconciled.

For any adapter or API behavior retained as implemented, update the smallest
current effects, execution, or standard-library specification alongside its
executable evidence. Do not describe prototype-only surfaces as available
production APIs. Keep the remaining production rollout in `filesystem-io.md`.

After all gates pass, review the parent proposal against that evidence before
moving it to Ready. Remove this completed proposal and its catalog entry,
replace the parent's dependency link with routes to the resulting current
specifications and checked evidence, and leave only unimplemented work in the
parent. Passing the gates does not itself complete the filesystem rollout.
