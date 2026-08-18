---
role: proposal
update-when: The agent-language-services trusted target guard, review-state transition, reviewed source-decision authority, frozen-inventory bootstrap allowlist, or lifecycle range-validation evidence changes.
---

# Agent Language Services Inventory Review Gate

## Summary

Establish the reviewed authority after the base-pinned PR check exists on the
default branch. This proposal remains blocked until
[Agent Language Services PR Target Guard](agent-language-services-pr-target-guard.md)
completes. Its only later selectable target is the `G0` to `G1` transition
below.

## Problem

Repeated frozen-inventory attempts passed validators that generated source
classification, lifecycle decisions, inventory leaves, and expected results
together. Agreement between those outputs proved only that one decision was
copied consistently. It did not prove that the decision matched the source.

The attempts also treated an ignored Markdown handoff as sufficient
authorization. Its adjacent checked target sidecar was absent or stale, and the
pull request did not expose that failure. A later attempt added the review gate
and the frozen inventory in one branch, so neither the reviewed authority nor
the target base preceded its consumer.

A content-only `validate` command cannot detect that history. The repository
needs one finite state transition contract whose inputs include the PR base,
head, event base branch, and independently resolved default branch.

## Review State Transition

After the trusted guard prerequisite merges, the validator recognizes exactly
these repository states:

| State | Required default-branch contents | Forbidden contents |
| --- | --- | --- |
| `G0` | The trusted target guard is complete and this active gate proposal is Ready. | A completed gate record, reviewed source-decision authority, or frozen lifecycle artifact. |
| `G1` | The completed gate record, reviewed source-decision authority, range validator, and the lifecycle proposal's `#frozen-source-universe` Ready route. | This active gate proposal or any frozen lifecycle artifact. |
| `G2` | Every `G1` authority plus tracked inventory-target provenance and the complete frozen artifact set. | A change from the `G1` bytes of the completed gate record or reviewed source-decision authority. |

The only permitted transitions are:

| Base | Head | Result |
| --- | --- | --- |
| `G0` | `G0` | Accept ordinary changes that do not stage a `G1` authority, inventory-target provenance, or a frozen lifecycle artifact. |
| `G0` | `G1` | Accept only the review-gate PR described below. |
| `G1` | `G2` | Accept only a later frozen-inventory PR whose target was issued from that exact `G1` commit. |
| `G0` | `G2` | Reject the combined gate-and-inventory history regardless of commit order. |
| `G1` | `G1` | Apply the immutable-authority guard when the changed paths select this workflow. |
| `G2` | `G2` | Apply the post-bootstrap immutable-artifact guard. |
| Any other pair | Any other pair | Reject the unrecognized lifecycle transition. |

For pull requests, `--event-base-ref` is the pull-request base branch name and
`--default-ref` resolves the repository default branch. The names must match,
and the event base commit, `--base`, and resolved default-branch commit must be
identical. A branch-local ancestor, local branch with the same name, or
temporary stacked base is not equivalent.

Direct pushes may not update the default branch. The trusted target guard
requires every change to enter through a pull request and pass the required
base-owned check. A post-push job is not an authorization mechanism.

The base-owned transition entry point is:

```text
node workflow-scripts/check-agent-language-services-transition.mjs validate-range --base <base-sha> --head <head-sha> --event-base-ref <base-name> --default-ref <default-ref> --receipt <receipt-json>
```

All range and receipt inputs are required. An absent, empty, or all-zero
revision is an error. The head-owned lifecycle validator may check proposed
content, but it cannot authorize a repository-state transition or change the
base-owned transition result.

## G0 To G1 Review Gate

This subsection is the complete target after the trusted guard prerequisite
merges. The PR starts from `G0` and ends at `G1`. It does not add
inventory-target provenance, the source-universe
contract, frozen inventory, lifecycle manifest, migration-ledger schema, or
migration-ledger fixture.

The gate PR installs the complete lifecycle content validator, its tests, and
its documentation-workflow registration before any frozen artifact exists.
Temporary fixtures cover the later artifact shapes and rejection corpus. The
merged `G1` bytes contain every executable policy input needed to validate
`G1 -> G2`; the inventory PR may add data but may not revise validator code,
tests, or workflow registration.

The reviewed source-decision authority is stored at:

```text
docs/reference/agent-language-services-lifecycle-review/source-decisions.json
```

The authority enumerates every non-frontmatter paragraph, complete CommonMark
list item including continuation lines, table row, and nonempty fenced line.
Each root records exact source text and digest plus one reviewed class of
`conformance` or `supporting`. Each semantic leaf records exact Unicode-scalar
spans and one reviewed lifecycle. Conformance leaves use `current`,
`completed`, or `planned`; supporting leaves may also use `removed`.

Every finite identity records its kind, literal name, root, leaf, and exact
span. Each identity span is contained by the union of spans for its named leaf;
root containment alone is insufficient. The checked identity sets include Q01
through Q22, exactly six saved workspace function-reference rows, every row
and named cell of both closed
matrices, every unresolved acceptance row, tool and resource kinds,
package-document declaration kinds, LSP encodings, and literal plugin
compatibility cells. Each set is checked for its exact reviewed membership, not
only for the presence of one member of its kind.

The structural parser may emit only source structure, text, spans, and digests.
No repository command may generate or overwrite `source_class`, finite
identities, lifecycle, or destination fields. A skeleton writer may emit only
the structural fields and must write to a path other than the reviewed
authority. Semantic decisions are reviewed data, not source-vocabulary output.

The review includes explicit goldens for these easily misclassified shapes:

| Source shape | Required decision |
| --- | --- |
| The implementation-status paragraph names exposed tools and remaining work. | Split current and planned clauses. |
| A continued list item extends definition and reference lookup beyond the implemented workspace symbol set. | Parse one complete list item and classify the extension as planned. |
| A planned clause mentions an implemented boundary. | Keep the planned requirement separate from the boundary statement. |
| A source-index requirement says that some cases are currently implemented. | Classify the generalization requirement as planned. |
| An implemented acceptance row contains case, result, and evidence cells. | Bind all current cells to the checked case that supports them. |
| An evidence cell contains implemented and planned evidence. | Split it into lifecycle-homogeneous leaves. |
| A future capability sentence uses the word `completed`. | Classify the requirement as planned. |

The `G0` to `G1` changed-path allowlist is closed:

```text
.github/workflows/workflow--test-scripts.yaml
docs/proposals/README.md
docs/proposals/agent-language-services-inventory-review-gate.md
docs/proposals/agent-language-services-lifecycle-migration.md
docs/reference/README.md
docs/reference/agent-language-services-lifecycle-review/source-decisions.json
docs/reference/implemented-proposals/README.md
docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md
docs/reference/proposal-target-receipts/<pull-request-number>.json
docs/reference/proposal-target-readiness/manifest.json
workflow-scripts/check-agent-language-services-lifecycle.mjs
workflow-scripts/check-agent-language-services-lifecycle.test.mjs
```

A rename, copy, deletion, or Git type change is a change to both the source and
destination paths. No frozen artifact path belongs to this allowlist.

### Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Invoke transition validation without every range input. | Reject before content validation and name the missing input. | Missing, empty, and all-zero argument cases. |
| Change ordinary documentation while the gate remains active. | Accept `G0 -> G0` only when the head adds no completed gate record, reviewed authority, target provenance, or frozen artifact. | Unrelated-document control plus one protected-path mutation for each later-state artifact family. |
| Validate an exact `G0` base and gate-only `G1` head. | Accept the gate transition and report `G0 -> G1`. | Temporary Git history with the default branch at the event base. |
| Complete the gate and add any frozen artifact in one branch. | Reject `G0 -> G2` regardless of commit ordering. | Combined-history fixture and one direct forbidden-path assertion per frozen artifact family. |
| Retarget the gate PR to a non-default or stacked branch. | Reject even when that branch name or commit matches another supplied input. | Distinct event-base and independently resolved default-ref fixtures. |
| Parse the unchanged umbrella proposal. | Cover every complete source root and meaningful Unicode scalar without semantic fields. | Parser result-shape, CommonMark continuation, separator, and non-BMP assertions. |
| Run every repository writer. | Preserve the reviewed authority byte-for-byte and emit no semantic field from a structural writer. | Writer non-mutation and output-shape assertions. |
| Review known mixed and ambiguous source shapes. | Match every golden decision in the table above without keyword inference. | One positive golden and one wrong or joined lifecycle mutation per row. |
| Remove or detach reviewed source data. | Reject a missing root, semantic leaf, supporting classification, identity occurrence, or span even when a consumer copy changes to match. | Independent mutation per record class and finite identity kind. |
| Move an identity occurrence outside its named leaf. | Reject even when the span remains inside the same root and synchronized consumer copies agree. | One cross-leaf span mutation plus one omitted member for every finite identity set. |
| Change an unrelated or frozen path in the gate PR. | Reject the closed `G0 -> G1` allowlist and name the path. | Add, modify, copy, rename, delete, and Git-type-change fixtures. |
| Run the trusted workflow for a gate path. | Invoke the base-owned transition validator with event, default-branch, and current receipt inputs. | `pull_request_target` registration assertion plus authority-only protected-path case. |
| Simulate a later exact `G1 -> G2` transition. | Accept only when tracked provenance, event base, and default-branch commit all equal the exact `G1` commit. | Accepted temporary Git history with a separately merged `G1`. |
| Omit or stale the later handoff. | Reject a missing checked `TARGET.json` before implementation and reject missing, stale, blocked, wrong-anchor, or wrong-prerequisite tracked provenance in the PR. | Target-readiness cases plus one range fixture per provenance field. |
| Change the `G1` authority together with a frozen consumer. | Reject the synchronized mutation using bytes read from the exact base commit. | Temporary Git history that changes both reviewed and generated copies. |
| Attempt a direct default-branch push. | Reject before the ref update through the required pull-request ruleset. | Checked active-rules response plus a direct-push rejection control. |

Each rejection fixture changes one required fact unless the case explicitly
tests a synchronized mutation. A fixture with multiple unrelated invalid fields
does not prove which invariant rejected the input.

### Completion Rule

The gate completes only when all acceptance rows pass. Move this page
to the implemented-proposal records, remove it from the proposal catalog, and
move only `agent-language-services-lifecycle-migration.md#frozen-source-universe`
to Ready. `G1` must be merged to the default branch before a new inventory
target and sidecar are issued. Transfer the current external merge-authority
gate from this manifest entry to that lifecycle entry so target issuance
rechecks the App, broker, repository lock, and rulesets from current external
state.

## G1 To G2 Inventory Handoff

The later frozen-inventory PR consumes `G1`; it does not revise it. Its tracked
provenance names the exact `G1` default-branch commit, proposal subsection, and
prerequisite set. The event base commit, provenance base commit, and
independently resolved default-branch commit must be identical.

The inventory bootstrap allowlist contains only:

```text
docs/proposals/README.md
docs/proposals/agent-language-services-lifecycle-migration.md
docs/reference/README.md
docs/reference/agent-language-services-lifecycle/**
docs/reference/implemented-proposals/README.md
docs/reference/implemented-proposals/agent-language-services-frozen-source-inventory.md
docs/reference/proposal-target-receipts/<pull-request-number>.json
docs/reference/proposal-target-readiness/manifest.json
```

The reviewed source-decision authority, other active proposals, completed gate
record, trusted transition policy, lifecycle content validator, validator tests,
workflow registrations, MCP harness, executable MCP fixtures, semantic
baselines, and unrelated documentation are outside this allowlist. The
lifecycle migration proposal owns the frozen artifacts and their content
acceptance model.

## Non-Goals

- Generating lifecycle labels or finite identities from source vocabulary.
- Adding or accepting frozen lifecycle artifacts in the `G0` to `G1` PR.
- Migrating `agent-language-services.md` content.
- Changing Veln language, MCP, LSP, compiler, or runtime behavior.
