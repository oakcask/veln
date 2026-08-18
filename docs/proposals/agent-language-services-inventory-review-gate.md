---
role: proposal
update-when: The agent-language-services target provenance, reviewed source-decision authority, frozen-inventory bootstrap contract, or lifecycle validator acceptance evidence changes.
---

# Agent Language Services Inventory Review Gate

## Summary

Establish review authorities that a later frozen-inventory implementation can
consume but cannot create or revise. This documentation and repository-policy
gate is the only selectable agent-language-services lifecycle work until it
completes.

## Problem

Repeated frozen-inventory attempts passed their own validators while preserving
incorrect lifecycle decisions. The attempts created source classification,
inventory leaves, lifecycle expectations, and mutation tests in one change.
Agreement between those outputs therefore proved only that the same decision
was copied consistently.

The checked target-readiness policy also relies on ignored files under
`prompts/`. CI can validate a sidecar when a caller supplies its path, but the
pull request does not expose whether the sidecar existed or passed before
implementation. A Markdown-only handoff can therefore reach review even though
repository instructions reject it.

The next inventory attempt needs two authorities that exist before its branch:
a pull-request-visible target provenance contract and a reviewed source-decision
artifact. The inventory implementation may consume those authorities. It may
not generate, replace, or reinterpret them.

## Scope

This proposal adds a checked gate for the later frozen-inventory bootstrap. It
does not add the frozen inventory, migration ledger schema, production ledger,
or migrated proposal destinations.

The gate defines:

- a tracked target-provenance format that binds an implementation PR to its
  proposal path, heading, target kind, exact default-branch base, and exact
  prerequisite set;
- a PR check that requires and validates that provenance whenever the frozen
  lifecycle artifact set is first added;
- a reviewed source-decision artifact for the unchanged
  `agent-language-services.md` source at the path shown below;
- a structural parser that checks source-node and Unicode-scalar coverage but
  cannot emit conformance, identity, lifecycle, or destination decisions; and
- a finite mutation matrix that proves the reviewed decisions are connected to
  exact source spans and cannot be replaced by detached name lists.

The tracked target provenance remains with the frozen review artifacts after
merge. It is not a transient `prompts/` file. The later bootstrap guard treats
it and the reviewed source-decision artifact as immutable inputs.

The reviewed source-decision path is:

```text
docs/reference/agent-language-services-lifecycle-review/source-decisions.json
```

It becomes immutable when this gate merges, before a frozen inventory exists.
Every PR and default-branch push that changes that artifact runs the lifecycle
workflow. A later proposal may supersede the
authority only by replacing this lifecycle migration plan; an ordinary
correction PR may not rewrite the inventory input.

The frozen-inventory bootstrap allowlist contains only:

```text
.github/workflows/workflow--test-scripts.yaml
docs/reference/README.md
docs/reference/agent-language-services-lifecycle/**
workflow-scripts/check-agent-language-services-lifecycle.mjs
workflow-scripts/check-agent-language-services-lifecycle.test.mjs
```

The lifecycle reference family includes the tracked target provenance and its
short route page.

The reviewed source-decision artifact is not in that allowlist. Neither are
proposal pages, implementation records, toolchain sources, executable cases,
semantic baselines, or unrelated documentation.

## Reviewed Source Decisions

The source-decision artifact enumerates every non-frontmatter paragraph,
complete CommonMark list item including continuation lines, table row, and
nonempty fenced line. Each root records its exact source digest and one of
`conformance` or `supporting`.

Every conformance root enumerates its semantic leaves. Every leaf records exact
Unicode-scalar spans and one lifecycle from `current`, `completed`, or
`planned`. Supporting leaves may additionally use `removed`. Separator spans
contain only whitespace or parser-identified Markdown table punctuation.

Every named finite identity records its kind, name, source root, leaf, and exact
span. This includes Q01 through Q22, the six saved-reference rows, both closed
matrices, every unresolved acceptance row, tool and resource kinds,
package-document declaration kinds, LSP encodings, and literal plugin
compatibility cells. A top-level list of names without source occurrences is
not an identity authority.

Semantic lifecycle decisions are reviewed data. The parser and artifact writer
must not infer them from words such as `implemented`, `current`, `planned`,
`future`, or `remaining`. The validator checks completeness, span ownership,
digest binding, and agreement with the reviewed data. It does not claim to
derive meaning from prose.

The review must explicitly cover these known mixed or easily misclassified
source shapes:

| Source shape | Required decision |
| --- | --- |
| The implementation-status paragraph that names exposed tools and work that remains planned. | Split current and planned clauses into separate leaves. |
| The continued list item for definition and reference lookup beyond the implemented workspace symbol set. | Parse the continuation as one list item and classify the remaining work as planned. |
| A planned clause that mentions an implemented boundary. | Keep the planned requirement separate from the implemented boundary statement. |
| An acceptance row with an implemented case, result, and evidence. | Classify all implemented cells as current and bind each evidence claim to its matching checked case. |
| An acceptance row whose evidence cell contains both implemented and planned evidence. | Split the evidence cell into lifecycle-homogeneous leaves. |
| A future capability sentence that uses the word `completed`. | Classify by the requirement's lifecycle, not by the isolated word. |

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Open a frozen-inventory bootstrap PR without tracked target provenance. | Reject the PR before artifact validation and name the missing provenance artifact. | PR-event fixture with the complete frozen artifact addition and no provenance. |
| Supply malformed, blocked, stale, stacked, wrong-base, wrong-heading, or wrong-prerequisite provenance. | Reject the PR and name the mismatched readiness fact. | One rejection fixture for each provenance field and base relation. |
| Supply valid provenance from the exact default-branch base after this gate completes. | Accept provenance and make the reviewed source-decision artifact available as an immutable input. | Accepted temporary Git history with the completed gate record on the merge base. |
| Parse the unchanged umbrella proposal. | Cover every complete source root and every meaningful Unicode scalar exactly once without emitting semantic fields. | Parser result-shape assertion plus root and scalar coverage checks. |
| Remove a source root, a semantic leaf, a supporting classification, or an identity occurrence from the reviewed authority. | Reject the artifact even when another reviewed file is changed to match. | Independent mutations for each record class. |
| Delete or detach one identity from each finite identity kind. | Reject the artifact and name the missing source-bound occurrence. | One mutation per identity kind, including both matrix kinds and unresolved rows. |
| Join lifecycle-different clauses or split a complete list item at a continuation line. | Reject the source-decision artifact. | The six required source-shape cases plus gap, overlap, out-of-range, non-BMP, and continuation-line mutations. |
| Change a reviewed lifecycle and copy the same change into a consumer artifact. | Reject the consumer because the immutable pre-branch authority differs. | Temporary Git history whose base has the reviewed decision and whose head changes both copies. |
| Change the reviewed source-decision authority after this gate merges but before an inventory exists. | Run the workflow and reject the immutable authority change. | Gate-complete, pre-inventory PR and push range fixtures plus an authority-only path-filter fixture. |
| Change only a frozen JSON artifact after bootstrap. | Run the workflow and reject the immutable-path change. | JSON-only path-filter fixture and post-bootstrap range test. |
| Merge the bootstrap to the default branch. | Validate the pre-push base against the new head without treating the new head as the old default state. | Push-event fixture for the bootstrap transition. |
| Add an unrelated source, workflow, documentation, or toolchain path to the bootstrap PR. | Reject every path outside the explicit bootstrap allowlist. | Add, copy, rename, delete, and Git-type-change fixtures inside and outside the allowlist. |

## Completion Rule

This gate completes only when all twelve acceptance rows pass. Move its
completed record out of `docs/proposals/` before returning the lifecycle
migration to Ready. The completed record must link the immutable reviewed
source-decision artifact and the checked target-provenance route.

## Non-Goals

- Generating lifecycle labels from source vocabulary.
- Adding or accepting the frozen inventory in the same PR as this gate.
- Migrating `agent-language-services.md` content.
- Changing Veln language, MCP, LSP, compiler, or runtime behavior.
