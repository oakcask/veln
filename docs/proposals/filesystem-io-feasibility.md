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

The implemented provider foundation, sequential direct-facade outcome matrix,
empty-request bypass, and complete-transfer outcome matrices are specified by
[Standard-library filesystem and byte IO](../specification/standard-library-filesystem-io.md).
That specification owns the completed direct sequential, `read_exact`,
`write_all`, and `copy` observations, F3 observations, and the restricted
owner-release boundary that F5 must reuse. This proposal does not restate
those current results, ordering rules, or preserved-state contracts.
The following sections of the filesystem proposal define the remaining
prototype obligations:

| Design input | Prototype obligation |
| --- | --- |
| [Outcomes and error translation](filesystem-io.md#outcomes-and-error-translation) | Complete the remaining direct-facade count validation and host-translation rows not covered by implemented empty-request bypass, sequential no-progress normalization, and complete-transfer helpers. |
| [Namespace, seek, and mutation transitions](filesystem-io.md#namespace-seek-and-mutation-transitions) | Observe successful and failed operations through bytes, cursors, bindings, and commit facts. |
| [Concurrent operations and cleanup](filesystem-io.md#concurrent-operations-and-cleanup) | Preserve per-open admission, release obligations, aliases, and failure precedence. |

Implement only the adapter and source/provider support needed to demonstrate
these remaining contracts. F4 retains every unimplemented outcome and
transition row in the linked contracts; direct-facade empty-request bypass and
no-progress normalization and the implemented `read_exact`, `write_all`, and
bounded `copy` matrices are no longer proposal work.
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
| F4: remaining transfer and mutation | Direct invalid-count validation, host translation, seek and positional IO, open modes, metadata and entries, namespace mutation, truncate, append, and sync have independent expected bytes, cursor, commit, and namespace observations. This includes unsupported rename and unknown host progress. | Shared companion conformance cases for fake and supported system behavior, with scripted failures and progress logs; focused real-host cases for native translation, unsupported atomic operations, and uncertain host outcomes. |
| F5: cleanup | Close failure facts, retry, reverse cleanup, primary/related failures, and owner exit with retained child aliases obey the linked cleanup table while reusing the implemented F3 restricted-release boundary. | Companion tests for `No`, `Yes`, and `Unknown` release facts and retry across the restricted boundary; backend owner-exit and cancellation cases; human and JSON projections for primary failure and ordered related cleanup failures. |

## Implementation Sequence And Evidence Placement

1. Apply the unimplemented outcome and transition rows to F4 and F5. Keep fake
   state and failure plans in Veln. Add host-boundary cases only for
   observations that require a real host or backend integration. Do not repeat
   the implemented empty-request, sequential direct-facade, or complete-transfer
   matrices as proposal evidence.

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

This proposal completes only when the remaining F4 rows and F5 have checked
evidence at the stated boundaries. Bounded `copy` and F3 are implemented and
specified independently. A failed remaining gate keeps the target incomplete
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
