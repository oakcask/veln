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

Do not select this proposal while
[Agent Language Services Inventory Review Gate](agent-language-services-inventory-review-gate.md#g0-to-g1-review-gate)
remains under `docs/proposals/`. That gate must complete the finite `G0` to
`G1` transition before another inventory implementation begins.

After the gate completes, select only the corrected frozen source inventory PR
first. Its base is the default branch revision that contains all completed
prerequisite records, the reviewed source-decision authority, and no frozen
inventory. The PR must carry checked tracked target provenance. It may consume
the reviewed source decisions but may not revise them. Keep all corrections on
that same default-branch-targeting PR. Do not put a correction PR on a branch
that already contains any frozen artifact. Select the migration PR only after
the frozen inventory has merged. Neither target may change the MCP harness,
executable MCP fixtures, or semantic baselines.

Generate the inventory target only after the gate completion is present on the
default branch. Discard any earlier Markdown target and sidecar. A commit inside
the inventory branch, including one exposed through a temporary pull-request
base branch, cannot substitute for the post-gate default-branch base.

The frozen-inventory PR is the `G1` to `G2` transition defined by the gate
proposal. CI must invoke its range-aware validator with the PR base, head,
event base branch, and independently resolved default branch. Content-only
artifact validation is not evidence that the transition is permitted.

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

Complete the migration in two PRs after the inventory review gate installs the
base-owned content validator and its tests. The first migration PR adds a
frozen source inventory, an independent source-universe contract, a lifecycle
review manifest, the migration-ledger schema, and the closed fixture corpus. It
does not revise validator code or reorganize or remove umbrella proposal
content. The second PR performs the lifecycle migration and may not modify the
frozen artifacts or validator policy.

The source-universe contract is derived from the immutable reviewed
source-decision authority established by the inventory review gate. Its root
records enumerate every non-frontmatter Markdown paragraph, complete CommonMark
list item including continuation lines, table row, and nonempty fenced line in
`agent-language-services.md`. Table cells and semantic clauses are child
records. Every record has a stable source ID and states whether it is
conformance content or supporting explanation.
Conformance children identify each normative paragraph, schema field, domain
error, resource template, lifecycle transition, capability-matrix row,
evidence-gate item, and client-platform cell. Supporting children remain
covered and identified but may not masquerade as conformance leaves.

Each contract identity records an exact source root, leaf, and Unicode-scalar
span. The contract records the exact expected identities for Q01 through Q22, the
six saved-reference rows, every row and named cell of both closed matrices,
every unresolved acceptance row, tool and resource kinds, package-document
declaration kinds, LSP encodings, and every literal plugin compatibility cell.
The validator uses a structural Markdown source parser that is independent of
the inventory generator. The parser derives the exhaustive root and table-cell
span set, but it does not assign conformance membership, named identity, or
lifecycle. The validator compares parser output with the source-universe
contract, then compares contract membership and named identities with the
inventory, in both directions. Deleting one source node from both the contract
and inventory must therefore fail. A generator may propose records, spans, or
digests, but generator output is not the completeness or lifecycle authority.
The validator also rejects duplicate and missing root IDs independently of
array length or encounter order. Replacing one expected root with a duplicate
of another must fail even when the inventory remains unchanged.

A detached top-level identity list is not sufficient evidence. Removing an
identity occurrence or changing a conformance record to supporting must fail
even when the inventory is changed to agree. A parser that turns one continued
list item into multiple source roots is invalid.

The structural parser result contains only source structure, text, spans, and
digests. It does not contain conformance, named-identity, or lifecycle fields.
The authoritative source-decision artifact precedes the inventory branch. The
source-universe contract and lifecycle manifest must equal that immutable input
where their fields overlap. The artifact writer does not create or overwrite
any reviewed input. A test invokes every writer mode and verifies that each
reviewed input remains byte-identical.

The inventory records the source heading, Unicode-scalar spans, exact text, and
digest for every source ID. Each parent scalar belongs exactly once to a child
span or a separator span. A separator span may contain only whitespace or
parser-identified table punctuation. Backticks, link text, link destinations,
hyphens in content, words, numbers, and code spans must belong to a child.
Every source-universe and inventory root carries its frozen exact text. Every
lifecycle-manifest leaf carries its exact span set. Omitting those fields while
retaining only a digest does not satisfy the frozen review contract.

When one source paragraph or row mixes completed and planned behavior, split
it into observable subrequirements in the inventory. The parent source ID
declares the exact subrequirement count. Child IDs use contiguous indices. Each
child records the exact Unicode-scalar spans that contain its source clause.
The child and separator spans must partition the parent source text without a
gap or overlap. The validator rejects a missing, duplicate, overlapping, or
out-of-range child, an invalid separator span, any uncovered source scalar, and
a parent mapped directly when it declares children.

Each leaf declares one lifecycle from the pre-branch reviewed source-decision
authority. A leaf may not combine implemented or current behavior with planned
or remaining behavior. The manifest records every conformance and supporting
leaf ID and span. It does not infer lifecycle from words such as `current`,
`implemented`, `completed`, `must`, or `exposes`. It also records which supporting leaves are
obsolete or duplicated and therefore use `removed`. Required golden decisions
include a planned clause that mentions an implemented boundary, a planned
prerequisite that mentions implemented history, a future completed capability,
and an implemented acceptance row whose case, result, and evidence share one
current lifecycle. The validator rejects any inventory or ledger lifecycle
that differs from the reviewed manifest.

Changing the reviewed authority and its consumer together in the inventory PR
must fail. The validator uses the authority from the declared merge base, not
the head revision, when it validates the bootstrap transition.

The inventory validator compares every recorded source digest with the
pre-migration umbrella source in the first PR. The migration validator in the
second PR treats the merged source-universe contract, lifecycle manifest,
inventory, schema, and validator acceptance corpus as immutable input. It does
not derive the expected universe from the edited umbrella page, edited ledger,
or the generator under test.

The immutable input includes the frozen exact source text or an equivalent
base-revision source snapshot. The second PR validates migrated destinations
against that input. It must not compare frozen digests with the reorganized
umbrella page and report the intended migration as source drift.

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

Each lifecycle PR adds exactly one tracked target receipt whose filename equals
the event PR number. The guard reads that receipt from the event head tree.
Existing receipts are immutable and are not lifecycle policy inputs for later
PRs.

The pull-request event base name is input to this check, not another candidate
for the repository default branch. The event base name must equal the
independently resolved default-branch name, and the event base commit,
provenance base commit, and resolved default-branch commit must be identical.
The check rejects a branch-local ancestor and a temporary non-default base even
when either contains the prerequisite records and reviewed authority.

After bootstrap, a separate immutability guard rejects changes to the frozen
source-universe contract, lifecycle manifest, inventory, schema, and acceptance
corpus. It also rejects changes to the validator implementation, its tests, and
the exact workflow step that registers the validator. Diff discovery includes
additions, copies, modifications, renames, deletions, and Git type changes.
Range tests cover the bootstrap transition, an unrelated documentation change
after bootstrap, the permitted second migration paths, a frozen-artifact edit,
a validator or workflow-registration edit, a protected-path rename, and a
regular file changed to a symbolic link.

Workflow path filters include every immutable JSON and provenance path. The
required pull-request ruleset prevents direct default-branch pushes.
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

Represent ledger entries as an object keyed by concrete inventory leaf ID. The
committed schema lists every leaf ID in `required` and rejects additional
properties. JSON object-key uniqueness and the closed key set therefore
express exactly-once structural coverage. Do not use an array whose
`uniqueItems` constraint compares whole entry objects instead of source IDs.

Reject a raw ledger object that repeats a source-ID key before ordinary JSON
parsing or schema evaluation. The strict-parser fixture is named
`duplicate_source_id_key` and contains exactly one repeated concrete leaf key
whose two values differ.

Execute the committed JSON Schema with a conforming schema evaluator. A
handwritten shape check is not evidence that the committed schema accepts or
rejects a fixture. Run the same closed structural corpus through the schema
evaluator and semantic validator. The schema-equivalence fixture identities
are:

```text
catch_all_leaf
invalid_destination_shape
invalid_removed_conformance
lifecycle_mismatch
missing_leaf
parent_mapping
range_leaf
unknown_leaf
wildcard_leaf
```

The validator rejects a missing, extra, or renamed required fixture before it
evaluates fixture contents. The schema-and-semantic equivalence claim is
limited to those nine structural fixtures plus the valid fixture. The strict
duplicate-key parser case is required but is outside schema equivalence because
it has no parsed JSON instance. Content
checks such as destination existence, role, anchor, and checked evidence remain
semantic-validator requirements and do not claim JSON Schema equivalence.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Complete the inventory review gate. | The default branch contains checked target provenance policy and an immutable reviewed source-decision authority before the inventory branch starts. | Completed gate record plus missing-provenance and synchronized-authority-mutation rejection fixtures. |
| Check target readiness before implementation. | The generated frozen-inventory target is accepted only after both prerequisites leave `docs/proposals/` and its declared default-branch base has no frozen artifact. | Accepted handoff from the checked target-readiness command plus blocked-prerequisite, stale-base, and stacked-base rejection fixtures. |
| Close the prerequisite client-platform set. | The platform-matrix proposal is complete and every plugin cell has a literal client-platform identity. | Link to the completed matrix record plus checked exact key list and row count. |
| Freeze the source universe independently. | A first PR records stable IDs, source spans, classifications, and digests for every independently parsed complete source node without changing the umbrella proposal or the pre-branch source-decision authority. The structural parser exposes no semantic classification, and no writer creates or overwrites reviewed inputs. | Source-parser result-shape assertion; CommonMark continuation fixture; writer non-mutation assertion; base-authority-to-contract and contract-to-inventory comparisons; injected source-node, contract-node, inventory-node, duplicate, unexpected, synchronized-mutation, and generator-omission cases for each record class. |
| Keep source-universe roots bijective. | Every expected reviewed root appears exactly once even when the root array length is unchanged. | Same-ID duplicate, omitted-root, and different-ID replacement fixtures. |
| Preserve named finite inputs. | Q01-Q22, both closed matrices, the six saved-reference rows, every unresolved acceptance row, and every other named conformance item have exact source-bound identity occurrences. | One missing and one detached source-occurrence mutation for each item class, including tool, resource, declaration, encoding, and plugin cells. |
| Separate lifecycle semantically. | Every conformance leaf matches the reviewed lifecycle manifest, and mixed parents partition every meaningful Unicode scalar without lifecycle mixing. | Golden ambiguous-word cases plus injected gap, overlap, out-of-range, wrong-lifecycle, hidden-delimiter, and non-BMP boundary failures. |
| Keep the ledger schema and validator equivalent. | The committed schema evaluator and semantic validator reject the nine named invalid structural fixtures and accept the same valid fixture. | Exact fixture-name check, keyed-entry schema, and closed-object cases. |
| Inventory the finite inputs. | The migration ledger contains exactly one entry for every frozen leaf ID and its lifecycle equals the frozen lifecycle. | Missing, strict duplicate-key, parent, wildcard, range, catch-all, unknown-leaf, and lifecycle-mismatch rejection cases. |
| Route current behavior. | Every `current` entry links to an existing specification anchor and unique checked evidence; no proposal or implementation record is cited as current authority. | Missing path, missing anchor, wrong role, wrong directory, empty evidence, duplicate evidence, missing evidence, and unchecked-evidence cases. |
| Preserve unresolved work. | Every `planned` entry retains a concrete input, observable outcome, boundary, and named evidence route in an active proposal page. | Ledger-to-proposal validation and proposal review. |
| Move completed history. | Every `completed` entry links to a supporting implementation record, and active proposal pages contain no implemented status ledger. | Stale implemented-row search and frontmatter validation. |
| Remove only supporting explanation. | Conformance leaves reject `removed`; supporting leaves accept it only with a rationale and existing superseding destination. | Paired conformance rejection and supporting-explanation acceptance fixtures. |
| Keep the completion gate finite. | Active proposal pages contain every planned leaf from the frozen inventory, including the closed matrices and Q01-Q22 identities, or link to focused proposal pages that contain them. No completion row depends on an undefined future matrix or capability list. | Frozen-inventory-to-proposal validation and injected missing-entry failure tests for each item class. |
| Bound the diff guard to its phase. | The bootstrap allowlist runs only on one default-branch-targeting PR whose event base commit equals its provenance base and independently resolved default-branch commit. A branch-local staged authority and any non-default or frozen stacked base are rejected. Later docs work passes while frozen artifacts, validator registration, and protected executable evidence remain immutable. | Base-ref and default-ref assertions plus the complete base/head PR event table and direct-push ruleset evidence, including missing-provenance, blocked-target, branch-local-authority, retargeted-non-default-base, stacked-base, JSON-only, type-change, copy, rename, unrelated-source, unrelated-doc, migration-path, frozen-edit, validator-edit, and workflow-registration cases. |
| Complete the migration independently. | The PR changes documentation and its documentation validators only. It does not change harness code, executable MCP fixtures, or semantic baselines. | Phase-aware diff-scope check with actionable failure output. |

## Non-Goals

- Implementing MCP JSONL assertions or changing an executable MCP case.
- Implementing saved references, broader navigation, resources, documentation
  publication, conformance, or plugins.
- Revising the membership of either closed matrix or Q01-Q22.
- Treating the migration ledger as current behavior authority.

## Completion Rule

This proposal completes only when all sixteen acceptance rows pass. Move the
completed proposal record out of `docs/proposals/` before selecting the MCP
JSONL assertion target. Later capability work may revise a closed set only
through a separate proposal that states the old and new finite membership.
