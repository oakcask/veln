---
role: proposal
update-when: The agent-language-services lifecycle-migration prerequisite, shared-capture evidence boundary, or saved-reference slice status changes.
---

# Agent Language Services Slice Closure

## Summary

Establish the remaining executable-evidence boundary for agent-language-service
slices before retrying saved workspace function references. After a separate,
lossless lifecycle migration completes, state when shared invariant evidence
can be composed with adapter evidence.

The saved-reference adapter is not implemented by this proposal. A later
target may reintroduce that bounded slice only after this proposal's closure
gate passes.

## Problem

The active agent-language-services proposal currently serves two roles. Its
lifecycle migration has a separate finite proposal because the closed
capability matrices, Q01 through Q22 gate, and unresolved acceptance rows must
not disappear while implemented history moves. Combining that migration with
harness work makes a lost requirement difficult to distinguish from an
intentional lifecycle edit.

The stable-capture invariant is shared across saved navigation adapters. The
current acceptance wording does not say whether every adapter must reproduce a
source mutation during capture or whether a shared invariant test plus an
adapter failure-mapping test is sufficient. That ambiguity encourages
adapter-specific tests that exercise a different failure trigger while being
described as source-mutation evidence.

## Goals

- Require the lossless agent-language-services lifecycle migration to complete
  before selecting the saved workspace function-reference slice.
- Define a compositional evidence rule for shared capture invariants and
  adapter-visible failure mapping.

## Non-Goals

- Implementing `references`, expanding the shared navigation symbol set, or
  changing Veln name resolution.
- Adding dependency or standard-library reference search, pagination, cursors,
  retained resources, documentation tools, or client plugins.
- Requiring every adapter to duplicate the same filesystem race harness.
- Treating implementation records or active proposals as current behavior.
- Reorganizing the umbrella agent-language-services proposal in the same PR as
  the harness and executable-evidence changes.

## Lifecycle Prerequisite

Complete [Agent Language Services Lifecycle Migration](agent-language-services-lifecycle-migration.md)
in a documentation-only PR before selecting this proposal's harness work. That
migration preserves the closed capability matrices, Q01 through Q22 gate, and
unresolved acceptance rows through an enumerated ledger. This proposal does
not repeat or weaken that gate.

## Shared Capture Evidence

A saved-navigation adapter may compose evidence across ownership boundaries
when all rows in this table are present:

| Claim | Required evidence owner |
| --- | --- |
| Source identity or bytes changing during capture cannot produce a successful stable snapshot. | The shared capture boundary has a deterministic mutation or identity-change test. |
| The adapter calls that shared boundary for the operation under review. | A focused adapter test or a narrow structural test identifies the route without replacing it with a weaker capture path. |
| Exhausted capture returns `snapshot_changed`. | The adapter-level tool result test asserts the domain code and `isError: true`. |
| Failure publishes no partial operation payload. | The same adapter-level result asserts that success-only fields are absent. |

An adapter does not need another timing-sensitive source mutation test when
these four rows are checked. If the adapter buffers or transforms partial
results before capture succeeds, it needs its own deterministic failure seam
and atomic-publication test.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Satisfy the lifecycle prerequisite. | The separate lifecycle-migration proposal is complete and no undefined future matrix replaces a preserved finite input. | Completed proposal record and its checked migration ledger. |
| Reuse the shared capture boundary from an adapter. | Shared mutation evidence and adapter mapping evidence jointly prove `snapshot_changed` with no partial success fields. | Shared capture test plus focused adapter result test, recorded together in the slice evidence map. |
| Complete this proposal. | Lifecycle migration and capture-evidence rules are complete; no requirement in this page remains planned. | Final proposal audit, then move this page to implemented proposal records and remove its catalog entry. |

## Implementation Order

1. Complete the documentation-only lifecycle-migration proposal.
2. Document the shared-capture evidence composition in the active proposal's
   acceptance map.
3. Complete the proposal lifecycle audit. A later proposal selection may then
   issue a new bounded saved-reference target.
