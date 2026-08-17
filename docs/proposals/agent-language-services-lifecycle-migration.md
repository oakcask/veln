---
role: proposal
update-when: The agent-language-services proposal structure, closed capability matrices, Q01-Q22 evidence gate, acceptance rows, or lifecycle destinations change.
---

# Agent Language Services Lifecycle Migration

## Summary

Separate implemented agent-language-service history from unresolved work
without replacing finite acceptance sets with summaries or future matrices.
This documentation-only migration is a prerequisite for the MCP JSONL
assertion target. It does not change toolchain behavior or executable cases.

## Selection State

This proposal is ready. Select only the frozen-source-inventory PR first. That
PR must add the inventory, migration-ledger schema, validator, and rejection
tests without reorganizing the umbrella proposal. Select the migration PR only
after the frozen inventory has merged. Neither target may change the MCP
harness, executable MCP fixtures, or semantic baselines.

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
  lifecycle transition, and supported client-platform cell in the conformance
  universe;
- the exact tool and resource kinds, package-document declaration kinds, LSP
  encodings, and plugin compatibility cells closed by the v1 manifest;
- the final requirement that the conformance gate reject missing, duplicate,
  skipped, orphaned, and undeclared mappings.

Condensing prose is allowed. Replacing any item above with an unnamed future
matrix, a capability category, or an unbounded phrase such as “remaining
symbols” is not allowed.

## Frozen Source Inventory

Complete the migration in two PRs. The first PR adds a frozen source inventory,
the migration-ledger schema, and their validator. It does not reorganize or
remove umbrella proposal content. The second PR performs the lifecycle
migration and may not modify the frozen inventory or weaken the validator.

The inventory is derived from `agent-language-services.md` as it exists before
the first PR. It assigns a stable source ID to each normative paragraph, table
row, schema field, domain error, resource template, lifecycle transition,
capability-matrix row, evidence-gate item, and client-platform cell. It records
the source heading and exact source text digest for every ID. Q01 through Q22,
the two closed matrices, tool and resource kinds, package-document declaration
kinds, LSP encodings, and plugin compatibility cells also retain their named
identity.

When one source paragraph or row mixes completed and planned behavior, split
it into observable subrequirements in the inventory. The parent source ID
declares the exact subrequirement count. Child IDs use contiguous indices. Each
child records the exact Unicode-scalar spans that contain its source clause.
The child spans must partition every non-whitespace, non-Markdown-delimiter
scalar in the parent source text without a gap or overlap. A delimiter span may
contain only table punctuation or whitespace; it may not hide a word, number,
code span, or link. The validator rejects a missing, duplicate, overlapping, or
out-of-range child, any uncovered source scalar, and a parent mapped directly
when it declares children.

Each child also declares the lifecycle stated by its source clause. A child may
not combine source text that states implemented or current behavior with text
that states planned or remaining behavior. A mixed parent must therefore have
at least one child for each source lifecycle it contains. The validator rejects
a child that contains lifecycle statements from more than one class and
rejects a parent lifecycle statement that is not covered by a matching child.

The inventory validator compares every recorded source digest with the
pre-migration umbrella source in the first PR. The migration validator in the
second PR treats the merged inventory as immutable input. It does not derive
the expected universe from the edited umbrella page or the edited ledger.

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

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Freeze the source universe independently. | A first PR records stable IDs and source digests for every item in the conformance universe without changing the umbrella proposal. Mixed-lifecycle source items declare child spans that completely partition their source text and separate each stated lifecycle. | Frozen-inventory validator, injected gap, overlap, wrong-lifecycle, missing-child, and changed-digest failures, plus a diff-scope check. |
| Inventory the finite inputs. | The migration ledger contains one entry for every leaf ID in the frozen inventory, including Q01-Q22, both closed matrices, the six saved-reference rows, every unresolved acceptance row, and every other conformance-universe item. | Review-record inspection plus checked duplicate-and-omission validation against the immutable inventory. |
| Route current behavior. | Every `current` entry links to specification and executable evidence; no proposal or implementation record is cited as current authority. | Link validation and lifecycle-class validation. |
| Preserve unresolved work. | Every `planned` entry retains a concrete input, observable outcome, boundary, and named evidence route in an active proposal page. | Ledger-to-proposal validation and proposal review. |
| Move completed history. | Every `completed` entry links to a supporting implementation record, and active proposal pages contain no implemented status ledger. | Stale implemented-row search and frontmatter validation. |
| Keep the completion gate finite. | Active proposal pages contain every planned leaf from the frozen inventory, including the closed matrices and Q01-Q22 identities, or link to focused proposal pages that contain them. No completion row depends on an undefined future matrix or capability list. | Frozen-inventory-to-proposal validation and injected missing-entry failure tests for each item class. |
| Complete the migration independently. | The PR changes documentation and its documentation validators only. It does not change harness code, executable MCP fixtures, or semantic baselines. | Diff-scope check. |

## Non-Goals

- Implementing MCP JSONL assertions or changing an executable MCP case.
- Implementing saved references, broader navigation, resources, documentation
  publication, conformance, or plugins.
- Revising the membership of either closed matrix or Q01-Q22.
- Treating the migration ledger as current behavior authority.

## Completion Rule

This proposal completes only when all seven acceptance rows pass. Move the
completed proposal record out of `docs/proposals/` before selecting the MCP
JSONL assertion target. Later capability work may revise a closed set only
through a separate proposal that states the old and new finite membership.
