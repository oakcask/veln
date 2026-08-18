---
role: proposal
update-when: The agent-language-services PR target receipt, trusted transition workflow, lifecycle protected paths, or target-pair metadata changes.
---

# Agent Language Services PR Target Guard

## Summary

Install one trusted pull-request guard before another agent-language-services
lifecycle transition. The guard copies the ignored target pair into a tracked
PR receipt and evaluates the transition with policy owned by the exact default-
branch base.

This bounded recovery has two ordered targets. The guard bootstrap is the only
initially selectable target. Guard activation and closure becomes selectable
only after the bootstrap is on the default branch. Both targets remain in
lifecycle state `G0`. Neither target completes the inventory review gate,
writes source decisions, or adds a frozen lifecycle artifact.

## Problem

The target-readiness validator rejects a missing, blocked, or stale target
sidecar only when an agent invokes it. Git ignores both handoff
files. A pull-request check cannot currently distinguish a validated target
pair from a Markdown-only handoff or no handoff.

A normal `pull_request` workflow also executes workflow and validator bytes
controlled by the pull-request head. A later commit can remove the check or
weaken a forbidden `G0 -> G2` result. An internal `G1` commit does not make that
head-owned policy or authority independent of its consumer.

The trusted guard must precede the `G0 -> G1` authority branch. The target pair
must become PR-visible, and the decision must remain unchanged when the head
rewrites its own workflow or validator.

## Target Pair And Receipt

Extend `target.schema.json` with a required `target_sha256` field. The target
generator writes `TARGET.md` first, computes SHA-256 over its exact UTF-8 bytes,
and stores the lowercase hexadecimal digest in `TARGET.json`. Generated target
Markdown begins at byte zero with LF-delimited YAML frontmatter and no
byte-order mark. The first line and the closing delimiter are exactly `---`. The
closed frontmatter contains each of these keys exactly once and no other key:

```text
target_schema_version: 1
proposal_path: <repository-relative proposal path>
proposal_anchor: <nonempty heading anchor>
base_commit: <full commit identity>
```

The parser rejects malformed YAML, duplicate keys, unknown keys, a second
identity envelope, and non-string values except the literal schema version.
The pair validator requires the other three values to equal the sidecar.
Human-readable links are derived from that identity and are not a second
selection authority.

The implementation branch adds exactly one JSON receipt under:

```text
docs/reference/proposal-target-receipts/<pull-request-number>.json
```

The receipt conforms to `pr-receipt.schema.json` and rejects duplicate object
keys and unknown fields. It contains:

- `schema_version`;
- the exact `TARGET.json` object;
- the exact `TARGET.md` text;
- repository identity and pull-request number;
- base repository, ref, and commit;
- head repository and ref; and
- receipt format and target-text encoding identifiers.

The target text uses a JSON string, so the validator can recover the exact
UTF-8 bytes and recompute `target_sha256`. The local emitter validates the
target pair before it writes the receipt. It receives the PR identity after PR
creation. The receipt path and `pull_request_number` must equal the event PR.
The receipt blob is read from the exact event head tree, which binds it to the
head commit without a self-referential commit field. Missing, duplicated, or
inconsistent inputs fail without writing a partial receipt.

Receipts are immutable historical provenance after merge. A later PR adds a
new numbered receipt; it may not modify, rename, replace, or delete an existing
receipt.

## Trusted Execution Boundary

Add `workflow--agent-language-services-target-guard.yaml` as a read-only
`pull_request_target` feedback workflow. It runs for `opened`, `reopened`,
`synchronize`, and `edited`. Its Actions check is advisory because it is
associated with the base revision and cannot bind authorization atomically to
one pull request. The workflow has only `contents: read` and
`pull-requests: read` permissions. It holds no App key or merge credential and
does not execute head code, install head dependencies, or source a head-owned
workflow, action, helper, schema, manifest, allowlist, or transition table.

The enforcing boundary is a repository-scoped GitHub App running in an
external merge broker. One active guard ruleset restricts updates to the
default branch and requires a pull request. Its only bypass actor is that App,
limited to pull-request merges; no user, administrator role, team, deploy key,
other App, or direct push can update the branch. Separate repository rulesets
retain their normal checks and reviews without an App bypass, so those
requirements still apply.

The App key remains outside the repository and GitHub Actions. The broker does
not accept a validation result or credential request from Actions or head code.
It serializes every default-branch merge for this repository under one lock,
then re-reads the current open PR, default-branch commit, head commit, target
receipt, active rules, and base-owned policy. It checks out the exact current
base in an isolated directory and supplies head Git objects and PR metadata
only as untrusted data. It reruns the transition validator and every transitive
policy input from that base. Immediately before invoking the merge API it
rechecks the lock, base, head, open PR identity, and rule identities, and passes
the exact expected head SHA to the merge operation. Because the App is the sole
default-branch updater and all its merges share the lock, the validated base
cannot change between that recheck and merge.

The App installation has only metadata read, contents write, and pull-request
read and write permissions. Bootstrap installs the broker coordinator and its
tests at the two named `workflow-scripts/` paths below. The deployed coordinator
digest must identify those exact default-branch bytes. Its repository lock
identity, installation identity, permission set, external key custody, and
base-policy invocation contract are external activation facts. Bootstrap
installs the repository-side policy and feedback workflow, but closure is not
selectable until authenticated broker-policy and repository-rules queries
prove those facts.

The transition validator installed by the bootstrap owns every
authorization-critical `G0 -> G1` invariant in the inventory review gate
acceptance model.
It reads the base umbrella source and the proposed reviewed authority as data.
A head-owned content validator or test cannot replace, weaken, or satisfy those
checks. After `G1` merges, the lifecycle content validator installed by the
gate becomes another base-owned policy input for `G1 -> G2`.

## Guard Bootstrap

This is the complete first target. Its base does not contain the trusted
workflow, so existing CI and adversarial review establish its completion
evidence. It installs the workflow, schemas, transition validator, and
target-pair validator. It keeps this proposal active and the inventory review gate
Blocked.

The guard-installation PR may change only:

```text
.github/workflows/workflow--agent-language-services-target-guard.yaml
docs/proposals/README.md
docs/proposals/agent-language-services-pr-target-guard.md
docs/reference/proposal-target-receipts/README.md
docs/reference/proposal-target-receipts/<pull-request-number>.json
docs/reference/proposal-target-readiness/README.md
docs/reference/proposal-target-readiness/manifest.json
docs/reference/proposal-target-readiness/manifest.schema.json
docs/reference/proposal-target-readiness/pr-receipt.schema.json
docs/reference/proposal-target-readiness/target.schema.json
workflow-scripts/check-agent-language-services-transition.mjs
workflow-scripts/check-agent-language-services-transition.test.mjs
workflow-scripts/broker-agent-language-services-merge.mjs
workflow-scripts/broker-agent-language-services-merge.test.mjs
workflow-scripts/check-proposal-target-readiness.mjs
workflow-scripts/check-proposal-target-readiness.test.mjs
```

The PR may add or modify the fifteen fixed paths and add exactly one receipt
whose filename equals the event PR number. Additions, copies, modifications,
renames, deletions, and Git type changes outside that set fail the bootstrap
scope check. Existing receipt files are immutable. In particular, the PR may
not add a completion record, the reviewed source-decision authority, inventory
target provenance, or any frozen lifecycle artifact.

On completion, rewrite this proposal to retain only the activation-and-closure
target. Change the Ready catalog and readiness manifest anchor from
`#guard-bootstrap` to `#guard-activation-and-closure`. Keep the inventory gate
Blocked. Extend that manifest entry with an external merge-authority gate that
names the exact default-branch target, pull-request rule, update restriction,
sole PR-only App bypass, separate no-bypass normal rules, App permissions,
broker release, repository lock, and key-custody policy.

## Guard Activation And Closure

After bootstrap merge, install the App and broker and activate the guard
ruleset described above. Use a separate no-bypass ruleset for the repository's
ordinary required checks, reviews, and up-to-date policy. An unrelated PR is a
pass-through case for ALS policy, but it still merges only through the broker
and after the ordinary rules pass.

The closure target emitter uses authenticated repository-rules and
broker-policy API queries whose callers can read complete bypass and deployment
facts. It refuses to emit a target until they prove the exact update
restriction, PR requirement, sole PR-only App bypass, separate no-bypass
ordinary rules, App installation and permissions, external key custody,
coordinator release digest, repository lock identity, and base-policy
invocation contract. The query results are rechecked before the closure PR and
summarized in the implementation record.

The base-guarded closure PR may change only:

```text
docs/proposals/README.md
docs/proposals/agent-language-services-inventory-review-gate.md
docs/proposals/agent-language-services-pr-target-guard.md
docs/reference/implemented-proposals/README.md
docs/reference/implemented-proposals/agent-language-services-pr-target-guard.md
docs/reference/proposal-target-readiness/manifest.json
docs/reference/proposal-target-receipts/<pull-request-number>.json
```

It moves this proposal to the implementation records, keeps the repository in
`G0`, and moves only the inventory review gate to Ready.

## State Model

| Base state | Receipt and range | Required result |
| --- | --- | --- |
| `G0` without the trusted guard | This guard-bootstrap target changes only its fifteen fixed paths and one new receipt. | Accept by bootstrap review, keep this proposal active, and remain `G0`. |
| Guarded `G0` without the required merge authority and rulesets | The closure target is requested. | Reject target issuance and keep the inventory gate Blocked. |
| Guarded `G0` with the required merge authority and rulesets | The closure target changes only its six fixed paths and one new receipt. | Accept with base policy, close this proposal, and move only the inventory gate to Ready. |
| Guarded `G0` | No lifecycle protected path changes. | Preserve `G0`; the broker applies ordinary repository rules without selecting an ALS target. |
| Guarded `G0` | A lifecycle protected path changes with zero, duplicate, malformed, stale, or replayed receipts. | Reject before head content validation. |
| Guarded `G0` | A receipt selects a blocked target or a base other than the exact event default-branch base. | Reject and preserve `G0`. |
| Guarded `G0` | One valid gate receipt and an exact gate-only range. | Permit base policy to evaluate `G0 -> G1`. |
| Guarded `G0` | One range contains gate completion and any `G2` artifact. | Reject `G0 -> G2` regardless of commit ordering. |
| `G1` | One valid inventory receipt from the exact merged `G1` base and unchanged trusted-policy paths. | Evaluate frozen artifacts with the base-owned lifecycle content validator. |
| `G1` | The range changes reviewed authority, gate record, trusted workflow, transition validator, lifecycle content validator, or their tests. | Reject before artifact validation. |

Lifecycle protected paths are the gate proposal and record, reviewed
source-decision authority, target provenance, every frozen lifecycle artifact
path, the trusted workflow, transition validator and tests, broker coordinator
and tests, every transitive runtime or deployment-policy input named by either
entry point, and the entire proposal-target-receipts directory including its
README and every numbered receipt. The base-owned transition validator contains
their exact path set before `G0 -> G1` begins.
At `G1`, the set also contains the lifecycle content validator, its tests, and
its workflow registration. A protected lifecycle PR may add exactly one new
receipt whose number equals that PR. The README and every receipt already
present in the base are immutable; ordinary and lifecycle PRs both reject their
modification, replacement, rename, or deletion.

Direct pushes may not update the default branch. The repository ruleset is the
preventive boundary: only the merge App can update it and the App can bypass
that restriction only for a pull-request merge. No post-push job is claimed to
undo an accepted push.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Validate a generated target pair. | Accept only one byte-zero identity envelope whose fields equal the sidecar and whose sidecar digest equals the exact Markdown bytes. | Valid pair plus missing file, byte-order mark, CRLF, missing envelope, duplicate envelope, malformed YAML, duplicate key, unknown key, wrong path, wrong anchor, wrong base, swapped Markdown, stale digest, and changed-byte fixtures. |
| Parse the tracked PR receipt. | Accept exactly one numbered receipt and recover both target files byte-for-byte. | Zero-receipt, duplicate-receipt, wrong-filename, malformed JSON, duplicate-key, unknown-field, and non-UTF-8 encoding cases. |
| Bind the receipt to the PR event. | Accept only exact repository, PR number, base ref and commit, head repository and ref, and the receipt blob from the event head tree. | One independent mismatch and replay fixture for every identity field. |
| Reuse a previously validated head in another PR. | Re-read current PR identity under the merge lock and reject the stale receipt even when the same head SHA passed earlier feedback. | Closed-PR, same-head different-PR, fork-ref, reopened, edited-base, and retarget controls. |
| Select a target from its declared base. | Accept only a Ready target whose prerequisites completed on that exact base. | Ready control plus blocked, unlisted, active-prerequisite, nonexistent-base, branch-local-base, and stale-base fixtures. |
| Rewrite or remove a head authorization input. | Continue to run base policy and reject the protected head change. | Head workflow deletion plus transition validator, broker coordinator, broker test, imported-helper, schema, manifest, allowlist, deployment-policy, and transition-table mutations, including synchronized coordinator and deployment-digest weakening. |
| Install the trusted guard. | Change only the fifteen fixed bootstrap paths and one new numbered receipt, add no later-state artifact, keep the gate Blocked, and preserve `G0`. | Exact allowed-path control plus receipt immutability and add, copy, modify, rename, delete, and Git-type-change failures outside the set. |
| Activate the merge broker and repository rulesets. | Keep the App key outside Actions, make the App the sole PR-only update-restriction bypass, leave ordinary requirements in a separate no-bypass ruleset, serialize merges, and let an unrelated conforming PR pass through. | Checked broker-policy, App-installation, and repository-rules API responses plus key-in-Actions, wrong App, extra bypass, direct push, concurrent merge, related-PR, and unrelated-PR controls. |
| Advance the default branch during validation. | Recheck under the repository lock and reject the stale base without merging. | Competing broker request, stale-base, changed-head, closed-PR, and updated-branch success controls. |
| Mutate historical receipt routes in an ordinary PR. | Reject changes to the receipt README and every base receipt even when no other lifecycle path changes. | Modify, replace, rename, delete, type-change, same-number overwrite, and unrelated-document controls. |
| Close the guard proposal. | Change only the six fixed closure paths and one new numbered receipt after merge-authority activation, transfer the external gate to the inventory entry, then move only the inventory gate to Ready. | Base-guarded closure range plus receipt, missing-record, missing-transferred-gate, wrong-manifest-anchor, extra-Ready-entry, and out-of-scope path failures. |
| Weaken a gate semantic check in the head. | Continue to apply every base-owned `G0 -> G1` authority invariant. | Missing root, wrong lifecycle, cross-leaf identity, omitted identity member, and synchronized head-validator weakening cases. |
| Complete only the inventory review gate after guard installation. | Accept exact `G0 -> G1` with one valid receipt and unchanged trusted-policy paths. | Separate temporary histories for the merged guard base and gate head. |
| Complete the gate and add frozen artifacts in one range. | Reject every commit ordering and every internal `G1` checkpoint. | Gate-first, inventory-first, and interleaved combined-history fixtures. |
| Mutate `G1` authority together with its consumer. | Reject using base bytes before comparing synchronized head copies. | Authority-only and synchronized authority-and-inventory mutations. |
| Change ordinary documentation outside lifecycle protected paths. | Preserve state without requiring an ALS target receipt. | Unrelated-document control. |

Each rejection fixture changes one required fact unless the case explicitly
tests replay, synchronized mutation, or combined history. The workflow and
broker contract tests assert base ownership for the entry point and every
transitive policy input.

## Completion Rule

The bootstrap completes only when its scope and acceptance rows pass on the
default branch. It does not complete this proposal. The closure completes only
after the checked broker, App installation, and ruleset evidence and all
remaining acceptance rows pass.

The closure moves this page to implemented-proposal records and removes its
catalog and manifest entries. It retains this proposal path as a completed
prerequisite on the inventory-gate manifest entry, transfers the identical
external merge-authority gate to that entry, adds the matching implementation
record, and moves only that gate entry to Ready. Every later ALS lifecycle
closure transfers the gate to its newly Ready successor. Target issuance
requeries the mutable broker, App, and ruleset facts every time; a historical
completion record never substitutes for that query. Generate a fresh gate
target pair from the resulting default-branch commit.

## Non-Goals

- Generalizing receipts beyond the agent-language-services lifecycle protected
  paths in this slice.
- Completing the inventory review gate or writing reviewed source decisions.
- Adding inventory provenance, a source universe, a lifecycle manifest, a
  migration-ledger schema, or ledger fixtures.
- Changing Veln language, MCP, LSP, compiler, or runtime behavior.
