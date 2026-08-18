---
role: proposal
update-when: The agent-language-services proposal structure, G0-G1-G2 review-state transition, target provenance, frozen source-universe contract, lifecycle review authority, migration-ledger schema, bootstrap branch topology, diff-guard phase boundary, or lifecycle destinations change.
---

# Agent Language Services Lifecycle Migration

## Summary

Separate implemented agent-language-service history from unresolved work
without replacing finite acceptance sets with summaries or future matrices.
This documentation-only migration is a prerequisite for the MCP JSONL
assertion target. It does not change toolchain behavior or executable cases.

## Selection State

The checked target-readiness prerequisite is recorded under
[Checked Proposal Target Readiness](../reference/implemented-proposals/checked-proposal-target-readiness.md).
The client-platform prerequisite is complete and recorded under
[Agent Language Services Platform Matrix Closure](../reference/implemented-proposals/agent-language-services-platform-matrix-closure.md).
The reviewed source-decision authority prerequisite is complete and recorded
under
[Agent Language Services Inventory Review Gate](../reference/implemented-proposals/agent-language-services-inventory-review-gate.md).
The frozen source inventory prerequisite is complete and recorded under
[Agent Language Services Frozen Source Inventory](../reference/implemented-proposals/agent-language-services-frozen-source-inventory.md).

Select only the content migration PR. Its base must already contain the frozen
source-universe contract, lifecycle manifest, inventory, migration-ledger
schema, validator, checked target provenance, and acceptance corpus. The
migration PR may add the production migration ledger and reorganize
`agent-language-services.md` into its destinations. It may not revise the frozen
artifacts, weaken the validator, or change the MCP harness, executable MCP
fixtures, or semantic baselines.

## Problem

The active agent-language-services proposal mixes implemented history with
planned behavior. A lifecycle split is necessary, but deleting the detailed
plan while moving the history would also delete the proposal's stopping
condition. In particular, the closed v1 navigation matrix, the closed v1
published-topic matrix, the Q01 through Q22 evidence gate, and unresolved
acceptance rows are finite inputs to later work. A reference to a matrix that
will be revised later is not an equivalent completion gate.

Combining this large documentation migration with a harness implementation
also makes an evidence regression hard to distinguish from an intentional
lifecycle edit. The migration must therefore complete independently before a
target changes MCP fixtures or harness assertions.

An earlier inventory attempt exposed three contract defects. The umbrella
proposal did not enumerate its platform set. The proposed generator also used
its own word-based lifecycle classifier as the expected result, so a mistaken
classification became self-validating. Finally, the first-PR diff guard had no
activation or retirement fact and would have rejected ordinary documentation
changes after merge. Repeating the generator or adding more same-shaped
mutation tests cannot correct those missing authorities.

Later attempts exposed the remaining authority defect. A checked sidecar under
ignored `prompts/` was not visible to the PR check. The inventory PR then
created its reviewed source classifications and their validator together.
Detached identity name lists, keyword-derived lifecycle labels, incomplete
CommonMark list items, and schema-versus-semantic disagreement all passed when
both sides copied the same error. The inventory review gate separates those
authorities from their later consumer.

## Scope

The migration classifies the current agent-language-services content into four
destinations.

| Source content | Required destination |
| --- | --- |
| Current observable behavior | The matching specification and executable example route. |
| Completed rationale and evidence routes | One or more supporting implementation records. |
| Unresolved externally observable requirements | Active proposal pages with the same finite acceptance identity. |
| Obsolete or duplicated explanation | Removal with one recorded rationale in the migration ledger. |

The stable `agent-language-services.md` path remains the short entry point. It
may route detailed unresolved work to focused proposal pages. A routed detail
page remains a proposal and stays in the proposal catalog until its own finite
acceptance set is complete.

## Preserved Finite Inputs

The migration must preserve these closed inputs until a separate proposal
explicitly revises them:

- the six acceptance rows for the saved workspace function-reference slice;
- every row of the closed v1 navigation symbol matrix;
- every row of the closed v1 published language-reference topic matrix;
- every named Q01 through Q22 evidence requirement and its rejection or
  boundary cases;
- every unresolved row under server and project selection, diagnostics and
  navigation, virtual locations and package documentation, published language
  reference, and plugin acceptance;
- every normative paragraph, schema field, domain error, resource template,
  lifecycle transition, and literal client-platform cell in the conformance
  universe after the platform-matrix prerequisite closes that set;
- the exact tool and resource kinds, package-document declaration kinds, LSP
  encodings, and plugin compatibility cells closed by the v1 manifest;
- the final requirement that the conformance gate reject missing, duplicate,
  skipped, orphaned, and undeclared mappings.

Condensing prose is allowed. Replacing any item above with an unnamed future
matrix, a capability category, or an unbounded phrase such as “remaining
symbols” is not allowed.

## Frozen Source Universe

The frozen source universe is complete. Its artifact route is
[Agent Language Services Lifecycle Artifacts](../reference/agent-language-services-lifecycle/README.md),
and its completion record is
[Agent Language Services Frozen Source Inventory](../reference/implemented-proposals/agent-language-services-frozen-source-inventory.md).

The migration PR treats the merged source-universe contract, lifecycle
manifest, inventory, schema, validator, and acceptance corpus as immutable
input. Authority corrections are allowed only in the frozen-inventory bootstrap
range that writes those artifacts and validates them as one acceptance corpus.
The migration PR must not derive the expected universe from the edited umbrella
page, edited ledger, or the generator under test. It validates migrated
destinations against the frozen exact source text and does not compare frozen
digests with the reorganized umbrella page as source drift.

## Diff-Guard Phases

The frozen-source allowlist guard is active only when the base revision does
not contain the frozen inventory and the head revision adds it. After that
bootstrap transition merges, the allowlist retires automatically. An unrelated
documentation PR and the second migration PR must not inherit the bootstrap
allowlist.

The inventory review gate defines the complete bootstrap allowlist. The
inventory implementation may not add another path or path family to that set.

The guard also receives the pull-request base ref and the default-branch ref.
The bootstrap transition is valid only when both refs name the default branch,
the base contains all completed prerequisite records, the reviewed
source-decision authority, and no frozen artifact. The head also contains valid
tracked target provenance for that exact base. A stacked base that contains a
frozen artifact is post-bootstrap even when the default branch does not contain
one. It cannot be used to revise the frozen artifacts or validator.

The pull-request event base name is input to this check, not another candidate
for the repository default branch. The event base name must equal the
independently resolved default-branch name, and the event base commit,
provenance base commit, and resolved default-branch commit must be identical.
The check rejects a branch-local ancestor and a temporary non-default base even
when either contains the prerequisite records and reviewed authority.

The post-bootstrap immutability guard rejects changes to the frozen
source-universe contract, lifecycle manifest, inventory, schema, and acceptance
corpus. It also rejects changes to the validator implementation, its tests, and
the exact workflow step that registers the validator. Diff discovery includes
additions, copies, modifications, renames, deletions, and Git type changes.
Range tests cover the bootstrap transition, an unrelated documentation change
after bootstrap, the permitted second migration paths, a frozen-artifact edit,
a validator or workflow-registration edit, a protected-path rename, and a
regular file changed to a symbolic link.

Workflow path filters include every immutable JSON and provenance path. Push
validation receives the pre-push default revision separately from the new head;
it does not use the new head as both states of the bootstrap transition.
Diff-scope validation examines every protected artifact, validator, test, and
workflow-registration path in the selected range. It does not return early
because no lifecycle JSON path changed. A local invocation without a concrete
base and head rejects the invocation instead of treating an empty discovered
path set as success.

| Base state | Head state | Required result |
| --- | --- | --- |
| A prerequisite remains active. | A frozen artifact is added. | Reject the blocked target before diff allowlisting. |
| A branch-local commit completes the prerequisite and adds the reviewed authority. | A later commit adds the frozen artifacts and declares the earlier commit as its base. | Reject the self-authored base even when a temporary pull-request base ref points to it. |
| The default branch has both completed prerequisites and no frozen artifact. | The first complete artifact set is added. | Apply the bootstrap allowlist. |
| A non-default stacked base already has a frozen artifact. | A frozen artifact or validator changes. | Reject the stack as a post-bootstrap mutation. |
| The default branch has the merged frozen artifact set. | Only permitted migration paths change. | Apply the post-bootstrap immutability guard and accept. |
| The default branch has the merged frozen artifact set. | A frozen artifact, validator, test corpus, or workflow registration changes. | Reject the immutable-path change. |

A scope failure names the changed path and tells the maintainer to restore it
or move the change to the permitted later PR. It also states whether the path
would invalidate the frozen review universe or mix toolchain behavior into a
documentation-only migration.

## Migration Ledger

Add one bounded review record for the migration. The record assigns each
preserved input to exactly one destination and one lifecycle class:
`current`, `completed`, `planned`, or `removed`. A `current` entry links to a
specification and checked evidence. A `completed` entry links to its supporting
implementation record. A `planned` entry links to the active proposal row that
retains its acceptance condition. A `removed` entry states why the source was
obsolete or duplicate. A leaf in the frozen conformance universe may not use
`removed`. Removal is available only for non-normative explanation outside that
universe, and it must link to the duplicate destination or superseding decision.

The ledger must map every leaf ID in the frozen source inventory. It must
enumerate Q01 through Q22 individually, each row of both closed matrices, and
every unresolved acceptance row. It must also map every other item in the
conformance universe, including schema, error, resource, lifecycle, bound,
declaration-kind, encoding, and client-platform items. No entry may use a
range, wildcard, or “all remaining rows” placeholder.

The migration is incomplete when a source item has no destination, has more
than one lifecycle class, or reaches only a summary that lacks its observable
outcome and planned evidence.

The ledger lifecycle must equal the frozen lifecycle for that leaf. A
conformance leaf may not use `removed`. A supporting-explanation leaf may use
`removed` only with a rationale and an existing superseding destination.

A valid destination resolves to an existing repository-relative Markdown path
and anchor. Its frontmatter role must match its lifecycle: `current` routes to
a specification and checked evidence, `completed` routes to an implementation
record, and `planned` routes to an active proposal row. Checked evidence
references must be nonempty, unique, resolve to an allowlisted executable or
checked-fixture route, and identify the case that supports the current claim.

Run one acceptance and rejection corpus through both the migration-ledger JSON
Schema and the semantic validator. The corpus mutates every required field,
closed-object boundary, source-ID pattern, lifecycle condition, destination
kind, path, anchor, evidence requirement, removal rationale, and format field.
The schema and semantic validator must make the same accept-or-reject decision
for every structural case.

The schema makes every structurally unconditional destination field required.
Its lifecycle-specific branches require `path`, `anchor`, checked `evidence`,
and removal `rationale` wherever the semantic contract requires them. Each
structural rejection case changes one field or one closed-object boundary. A
case with multiple invalid fields does not demonstrate schema and semantic
equivalence for any one of them.

The corpus may not declare schema and semantic disagreement as an expected
success. Production ledger validation resolves destinations, roles, anchors,
and checked evidence against the repository. A corpus source ID must be an
actual leaf from the reviewed inventory; positive cases may not use unknown or
parent IDs.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Complete the inventory review gate. | Complete. The default branch contains checked target provenance policy and an immutable reviewed source-decision authority before the inventory branch starts. | [Agent Language Services Inventory Review Gate](../reference/implemented-proposals/agent-language-services-inventory-review-gate.md). |
| Check target readiness before implementation. | Complete. The generated frozen-inventory target is accepted only after both prerequisites leave `docs/proposals/` and its declared default-branch base has no frozen artifact. | [Checked Proposal Target Readiness](../reference/implemented-proposals/checked-proposal-target-readiness.md) and [Agent Language Services Frozen Source Inventory](../reference/implemented-proposals/agent-language-services-frozen-source-inventory.md). |
| Close the prerequisite client-platform set. | Complete. The platform-matrix proposal is complete and every plugin cell has a literal client-platform identity. | [Agent Language Services Platform Matrix Closure](../reference/implemented-proposals/agent-language-services-platform-matrix-closure.md). |
| Freeze the source universe independently. | Complete. The frozen artifacts record stable IDs, source spans, classifications, and digests for every independently parsed complete source node without changing the umbrella proposal or the pre-branch source-decision authority. | [Agent Language Services Frozen Source Inventory](../reference/implemented-proposals/agent-language-services-frozen-source-inventory.md). |
| Preserve named finite inputs. | Complete. Q01-Q22, both closed matrices, the six saved-reference rows, every unresolved acceptance row, and every other named conformance item have exact source-bound identity occurrences. | [Agent Language Services Lifecycle Artifacts](../reference/agent-language-services-lifecycle/README.md). |
| Separate lifecycle semantically. | Complete. Every conformance leaf matches the reviewed lifecycle manifest, and mixed parents partition every meaningful Unicode scalar without lifecycle mixing. | [Agent Language Services Frozen Source Inventory](../reference/implemented-proposals/agent-language-services-frozen-source-inventory.md). |
| Keep the ledger schema and validator equivalent. | Complete for the schema fixture corpus. Both validators reject the same invalid structural ledger cases and accept the same valid corpus. | [Agent Language Services Lifecycle Artifacts](../reference/agent-language-services-lifecycle/README.md). |
| Inventory the finite inputs. | The migration ledger contains exactly one entry for every frozen leaf ID and its lifecycle equals the frozen lifecycle. | Missing, duplicate, parent, wildcard, range, catch-all, unknown-leaf, and lifecycle-mismatch rejection cases. |
| Route current behavior. | Every `current` entry links to an existing specification anchor and unique checked evidence; no proposal or implementation record is cited as current authority. | Missing path, missing anchor, wrong role, wrong directory, empty evidence, duplicate evidence, missing evidence, and unchecked-evidence cases. |
| Preserve unresolved work. | Every `planned` entry retains a concrete input, observable outcome, boundary, and named evidence route in an active proposal page. | Ledger-to-proposal validation and proposal review. |
| Move completed history. | Every `completed` entry links to a supporting implementation record, and active proposal pages contain no implemented status ledger. | Stale implemented-row search and frontmatter validation. |
| Remove only supporting explanation. | Conformance leaves reject `removed`; supporting leaves accept it only with a rationale and existing superseding destination. | Paired conformance rejection and supporting-explanation acceptance fixtures. |
| Keep the completion gate finite. | Active proposal pages contain every planned leaf from the frozen inventory, including the closed matrices and Q01-Q22 identities, or link to focused proposal pages that contain them. No completion row depends on an undefined future matrix or capability list. | Frozen-inventory-to-proposal validation and injected missing-entry failure tests for each item class. |
| Bound the diff guard to its phase. | The bootstrap allowlist runs only on one default-branch-targeting PR whose event base commit equals its provenance base and independently resolved default-branch commit. A branch-local staged authority and any non-default or frozen stacked base are rejected. Later docs work passes while frozen artifacts, validator registration, and protected executable evidence remain immutable. | Base-ref and default-ref assertions plus the complete base/head and PR/push event table, including missing-provenance, blocked-target, branch-local-authority, retargeted-non-default-base, stacked-base, JSON-only, type-change, copy, rename, unrelated-source, unrelated-doc, migration-path, frozen-edit, validator-edit, and workflow-registration cases. |
| Complete the migration independently. | The PR changes documentation and its documentation validators only. It does not change harness code, executable MCP fixtures, or semantic baselines. | Phase-aware diff-scope check with actionable failure output. |

## Non-Goals

- Implementing MCP JSONL assertions or changing an executable MCP case.
- Implementing saved references, broader navigation, resources, documentation
  publication, conformance, or plugins.
- Revising the membership of either closed matrix or Q01-Q22.
- Treating the migration ledger as current behavior authority.

## Completion Rule

This proposal completes only when all fifteen acceptance rows pass. Move the
completed proposal record out of `docs/proposals/` before selecting the MCP
JSONL assertion target. Later capability work may revise a closed set only
through a separate proposal that states the old and new finite membership.
