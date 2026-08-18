---
role: proposal
update-when: The agent-language-services proposal structure, client-platform matrix, frozen source-universe contract, lifecycle review manifest, migration-ledger schema, diff-guard phase boundary, or lifecycle destinations change.
---

# Agent Language Services Lifecycle Migration

## Summary

Separate implemented agent-language-service history from unresolved work
without replacing finite acceptance sets with summaries or future matrices.
This documentation-only migration is a prerequisite for the MCP JSONL
assertion target. It does not change toolchain behavior or executable cases.

## Selection State

The platform-matrix prerequisite is complete in
[Agent Language Services Platform Matrix Closure](../reference/implemented-proposals/agent-language-services-platform-matrix-closure.md).
Select only the corrected frozen source inventory PR first. That PR must add
the source-universe contract, inventory, lifecycle review manifest,
migration-ledger schema, validator, and rejection tests without reorganizing
the umbrella proposal. Select the migration PR only after the frozen inventory
has merged. Neither target may change the MCP harness, executable MCP fixtures,
or semantic baselines.

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
  universe identified by
  [Closed Client-Platform Matrix](agent-language-services.md#closed-client-platform-matrix "matrix-ref:lifecycle-conformance-cells");
- the exact tool and resource kinds, package-document declaration kinds, LSP
  encodings, and plugin compatibility cells identified by
  [Closed Client-Platform Matrix](agent-language-services.md#closed-client-platform-matrix "matrix-ref:lifecycle-compatibility-cells");
- the final requirement that the conformance gate reject missing, duplicate,
  skipped, orphaned, and undeclared mappings.

Condensing prose is allowed. Replacing any item above with an unnamed future
matrix, a capability category, or an unbounded phrase such as “remaining
symbols” is not allowed.

## Frozen Source Universe

Complete the migration in two PRs. The first PR adds a frozen source inventory,
an independent source-universe contract, a lifecycle review manifest, the
migration-ledger schema, and their validator. It does not reorganize or remove
umbrella proposal content. The second PR performs the lifecycle migration and
may not modify the frozen artifacts or weaken the validator.

The source-universe contract is independent of the inventory generator. Its
root records enumerate every non-frontmatter Markdown paragraph, list item,
table row, and nonempty fenced line in `agent-language-services.md`. Table
cells and semantic clauses are child records. Every record has a stable source
ID and states whether it is conformance content or supporting explanation.
Conformance children identify each normative paragraph, schema field, domain
error, resource template, lifecycle transition, capability-matrix row,
evidence-gate item, and client-platform cell. Supporting children remain
covered and identified but may not masquerade as conformance leaves.

The contract records the exact expected identities for Q01 through Q22, the
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

The inventory records the source heading, Unicode-scalar spans, exact text, and
digest for every source ID. Each parent scalar belongs exactly once to a child
span or a separator span. A separator span may contain only whitespace or
parser-identified table punctuation. Backticks, link text, link destinations,
hyphens in content, words, numbers, and code spans must belong to a child.

When one source paragraph or row mixes completed and planned behavior, split
it into observable subrequirements in the inventory. The parent source ID
declares the exact subrequirement count. Child IDs use contiguous indices. Each
child records the exact Unicode-scalar spans that contain its source clause.
The child and separator spans must partition the parent source text without a
gap or overlap. The validator rejects a missing, duplicate, overlapping, or
out-of-range child, an invalid separator span, any uncovered source scalar, and
a parent mapped directly when it declares children.

Each leaf declares one lifecycle from an independently reviewed manifest. A
leaf may not combine implemented or current behavior with planned or remaining
behavior. The manifest records every conformance and supporting leaf ID and
span; it does not infer lifecycle from words such as `current`, `implemented`,
`completed`, `must`, or `exposes`. It also records which supporting leaves are
obsolete or duplicated and therefore use `removed`. Required golden decisions
include a planned clause that mentions an implemented boundary, a planned
prerequisite that mentions implemented history, a future completed capability,
and an implemented acceptance row whose case, result, and evidence share one
current lifecycle. The validator rejects any inventory or ledger lifecycle
that differs from the reviewed manifest.

The inventory validator compares every recorded source digest with the
pre-migration umbrella source in the first PR. The migration validator in the
second PR treats the merged source-universe contract, lifecycle manifest,
inventory, schema, and validator acceptance corpus as immutable input. It does
not derive the expected universe from the edited umbrella page, edited ledger,
or the generator under test.

## Diff-Guard Phases

The frozen-source allowlist guard is active only when the base revision does
not contain the frozen inventory and the head revision adds it. After that
bootstrap transition merges, the allowlist retires automatically. An unrelated
documentation PR and the second migration PR must not inherit the bootstrap
allowlist.

After bootstrap, a separate immutability guard rejects changes to the frozen
source-universe contract, lifecycle manifest, inventory, schema, and acceptance
corpus. It also rejects changes to the validator implementation, its tests, and
the exact workflow step that registers the validator. Diff discovery includes
additions, copies, modifications, renames, deletions, and Git type changes.
Range tests cover the bootstrap transition, an unrelated documentation change
after bootstrap, the permitted second migration paths, a frozen-artifact edit,
a validator or workflow-registration edit, a protected-path rename, and a
regular file changed to a symbolic link.

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

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Close the prerequisite client-platform set. | The platform-matrix proposal is complete and every plugin cell has a literal client-platform identity. | [Closed Client-Platform Matrix](agent-language-services.md#closed-client-platform-matrix "matrix-ref:lifecycle-prerequisite-acceptance"), the completed matrix record, and checked exact key list and row count. |
| Freeze the source universe independently. | A first PR records stable IDs, source spans, classifications, and digests for every independently parsed source node without changing the umbrella proposal. The generator is not the expected-universe authority. | Source-parser-to-contract and contract-to-inventory comparisons; injected source-node, contract-node, inventory-node, duplicate, unexpected, and generator-omission cases for each record class. |
| Preserve named finite inputs. | Q01-Q22, both closed matrices, the six saved-reference rows, every unresolved acceptance row, and every other named conformance item have exact independent identity sets. | One missing-identity mutation for each item class, including tool, resource, declaration, encoding, and plugin cells. |
| Separate lifecycle semantically. | Every conformance leaf matches the reviewed lifecycle manifest, and mixed parents partition every meaningful Unicode scalar without lifecycle mixing. | Golden ambiguous-word cases plus injected gap, overlap, out-of-range, wrong-lifecycle, hidden-delimiter, and non-BMP boundary failures. |
| Keep the ledger schema and validator equivalent. | Both validators reject the same invalid structural ledger cases and accept the same valid corpus. | Per-keyword schema weakening and closedness mutation corpus. |
| Inventory the finite inputs. | The migration ledger contains exactly one entry for every frozen leaf ID and its lifecycle equals the frozen lifecycle. | Missing, duplicate, parent, wildcard, range, catch-all, unknown-leaf, and lifecycle-mismatch rejection cases. |
| Route current behavior. | Every `current` entry links to an existing specification anchor and unique checked evidence; no proposal or implementation record is cited as current authority. | Missing path, missing anchor, wrong role, wrong directory, empty evidence, duplicate evidence, missing evidence, and unchecked-evidence cases. |
| Preserve unresolved work. | Every `planned` entry retains a concrete input, observable outcome, boundary, and named evidence route in an active proposal page. | Ledger-to-proposal validation and proposal review. |
| Move completed history. | Every `completed` entry links to a supporting implementation record, and active proposal pages contain no implemented status ledger. | Stale implemented-row search and frontmatter validation. |
| Remove only supporting explanation. | Conformance leaves reject `removed`; supporting leaves accept it only with a rationale and existing superseding destination. | Paired conformance rejection and supporting-explanation acceptance fixtures. |
| Keep the completion gate finite. | Active proposal pages contain every planned leaf from the frozen inventory, including the closed matrices and Q01-Q22 identities, or link to focused proposal pages that contain them. No completion row depends on an undefined future matrix or capability list. | Frozen-inventory-to-proposal validation and injected missing-entry failure tests for each item class. |
| Bound the diff guard to its phase. | The bootstrap allowlist runs only for the inventory-addition transition; later docs work passes while frozen artifacts, validator registration, and protected executable evidence remain immutable. | Base/head phase table with type-change, rename, unrelated-doc, migration-path, frozen-edit, validator-edit, and workflow-registration cases. |
| Complete the migration independently. | The PR changes documentation and its documentation validators only. It does not change harness code, executable MCP fixtures, or semantic baselines. | Phase-aware diff-scope check with actionable failure output. |

## Non-Goals

- Implementing MCP JSONL assertions or changing an executable MCP case.
- Implementing saved references, broader navigation, resources, documentation
  publication, conformance, or plugins.
- Revising the membership of either closed matrix or Q01-Q22.
- Treating the migration ledger as current behavior authority.

## Completion Rule

This proposal completes only when all thirteen acceptance rows pass. Move the
completed proposal record out of `docs/proposals/` before selecting the MCP
JSONL assertion target. Later capability work may revise a closed set only
through a separate proposal that states the old and new finite membership.
