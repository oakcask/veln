---
role: proposal
update-when: The bounded predicate work-budget investigation question, synthetic proof-model workload, accounting candidates, or completion and parent-review conditions change.
---

# Predicate Work-budget Feasibility

Determine a bounded, deterministic work policy for
[Typed Predicate Logic and Flow Facts](typed-predicate-facts.md). This is a
Ready investigation target. It can be completed with an isolated truth/order
model and generated fact/query sequences; it does not require a production
verifier, callable-contract syntax, or static-contract implementation.

## Question and decision consequences

The parent requires useful mandatory proofs and permits `Unknown` when work is
exhausted. The unresolved question is which mandatory-case boundary and
accounting policy preserve those guarantees at bounded work across independent
queries, repeated summaries, and recursive-component-sized query groups.

| Finding | Design consequence to review in the parent |
| --- | --- |
| The candidate mandatory cases fit a deterministic bound within an explicitly stated input domain | Specify that domain, mandatory cases, accounting scope/unit, default limit, and policy-version behavior. |
| One query is bounded but many queries or retained facts exceed the group bound | Add aggregate accounting or revise its scope; a per-query limit alone is insufficient. |
| A candidate mandatory family requires uncontrolled search or retention | Narrow that family's mandatory guarantee or apply an explicit `Unknown` cutoff; do not silently declare it proved. |
| Cold/warm execution or declaration order changes the logical outcome under the same policy | Revise accounting/cache treatment so the policy is deterministic before recommending it. |

This evidence can change which otherwise valid static programs are accepted.
It is not a request to implement every parent transfer case or to prove the
complete static-contract feature before its selection.

## Bounded investigation and acceptance

Use the parent's proposed Boolean and signed 64-bit comparison model without
arithmetic terms, arbitrary predicate calls, a new solver dependency, or
production compiler integration. Resolved immutable identities and record
paths may be supplied directly to the isolated model. Expected truth/order
results come from an independent model, not the candidate proof procedure.

| Input or comparison | Required observation |
| --- | --- |
| Literal comparisons, Boolean identities, copy equality, substitution, transitivity, strict-to-inclusive weakening, complementary conditions, and signed endpoints | Identify the exact mandatory cases and their supported input domain; compare outcomes with the independent truth/order oracle. |
| Long Boolean formulas and deep alias/record paths at doubled bounded sizes | Record input size, proof steps, retained facts, and peak retention; demonstrate a non-exponential bound or a deterministic cutoff. |
| Repeated queries and synthetic call-summary obligations, individually and in groups | Account for query work and retained state across the proposed scope; show why the proposed limit cannot be bypassed by splitting work into calls. |
| A query just before and at the selected exhaustion boundary | Produce the specified established outcome or `Unknown` with a budget reason; exhaustion never creates proof authority. |
| Identical queries with cold/warm model state and reversed declaration order | Preserve outcomes under the same candidate policy and explain how caches are charged or normalized. |
| Candidate proof work versus a control that constructs the same identities but performs no proof search | Separate model/input cost from proof steps; elapsed time is supporting evidence rather than a portable pass threshold. |

Choose finite input caps before execution, record the workload and counters,
and use `bash scripts/agent-run` with the repository performance-audit workflow
for generated runs. This proposal defines no numerical defaults without
measurements. Model tests belong beside their isolated implementation, with
independently prepared success, failure, and exhaustion observations. They
must not expose unchecked summaries through production language features.

## Design basis and completion

The parent's [mandatory fragment](typed-predicate-facts.md#proposed-predicate-and-flow-model)
and [outcome model](typed-predicate-facts.md#entailment-outcomes) are the design
inputs; its related-work comparisons motivate a bounded comparison model but
do not supply numerical limits for Veln. The
[static-contract integration requirements](static-contracts.md#prerequisite-decisions)
identify synthetic consumer-shaped workloads without requiring that consumer
to exist. Retain the existing eager evaluation, value-identity, and trust
boundaries throughout the investigation.

Complete this target when the observations support a reviewable recommendation
for accounting scope, unit, defaults, versioning, mandatory-case boundaries,
and cutoff behavior, including any proposed narrowing and its user-visible
cost. A negative finding is a valid result if it identifies which guarantee
must change and why. Incorporate the reviewed decision into the parent before
considering its Ready promotion; investigation completion does not promote it
automatically. Remove this proposal and its catalog entry once that decision
and its maintained model evidence have an appropriate authority. Production
predicate transfer, callable compatibility, static-call enforcement, and their
full acceptance suites remain with their owning proposals.
