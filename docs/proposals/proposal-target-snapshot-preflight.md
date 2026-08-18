---
role: proposal
update-when: The proposal implementation target format, pull-request base identity source, Ready selection rule, proposal-review workflow, or repository agent-skill layout changes.
---

# Proposal Target Snapshot Preflight

## Summary

Add a repository-local preflight for work that claims to implement or review an
external `TARGET.md` snapshot. The preflight rejects a target whose
declared base, selected Ready proposal, or acceptance text predates the pull
request base.

The target remains external review input. It does not become repository merge
authority, tracked provenance, or a CI prerequisite.

## Problem

A platform-matrix target was issued from one default-branch commit. A later PR
corrected the selected proposal's compatibility-value boundary and expanded
its acceptance model. The next implementation used the corrected proposal and
newer PR base while the local target still named the old base and old contract.
Focused implementation tests could not detect that the work no longer
implemented the target presented to the reviewer.

The repository Ready catalog correctly owned proposal selection. The missing
guardrail was procedural: target-driven work did not compare its external
snapshot with the PR's repository base before implementation or acceptance.
Making ignored prompt files authoritative in CI would cross that ownership
boundary and repeat the reverted target-receipt design.

## Scope

Add a `proposal-target-snapshot-audit` repository skill. Its checker uses this
repository-relative path:

```text
.agents/skills/proposal-target-snapshot-audit/scripts/check-target-snapshot.mjs
```

Add one concise `AGENTS.md` rule that requires the skill whenever a task or
pull request claims to implement an external `TARGET.md` snapshot.

The checker runs with this exact interface:

```text
node .agents/skills/proposal-target-snapshot-audit/scripts/check-target-snapshot.mjs --target <path> --candidate-base <full-object-id> --output json
```

It exits zero and writes one LF-terminated JSON object to standard output on
success. It exits nonzero, writes no standard output, and writes one actionable
diagnostic to standard error on failure. The success object rejects duplicate
and unknown keys and has exactly this shape:

```json
{
  "schema_version": 1,
  "target_base": "<full-object-id>",
  "candidate_base": "<full-object-id>",
  "proposal_path": "docs/proposals/<name>.md",
  "proposal_anchor": "<empty-or-heading-anchor>",
  "proposal_blob_oid": "<full-object-id>",
  "catalog_blob_oid": "<full-object-id>",
  "identity_result": "pass"
}
```

The skill supplies the candidate base. Before a pull request exists, it uses
the remote named by the current branch configuration. If that value is absent,
it accepts the only configured remote and rejects zero or multiple remotes. It
queries that remote's symbolic `HEAD` with `git ls-remote --symref`, requires one
default branch and advertised commit, and fetches that exact branch. The
fetched default tip must equal the advertised commit. The skill runs
`git merge-base --all HEAD <default-tip>`, requires exactly one result equal to
the default tip, and uses the default tip as `candidate_base`.

During review, the skill reads `headRefOid` and `baseRefOid` from the pull
request and fetches both objects. It runs `git merge-base --all <headRefOid>
<baseRefOid>`, requires exactly one result equal to `baseRefOid`, and uses
`baseRefOid` as `candidate_base`. A missing default ref, changed advertisement,
missing object, shallow-history parent, stale branch, or zero or multiple merge
bases fails before the checker runs.

The target identity is the exact first five LF-delimited lines with no byte
order mark or leading bytes:

```text
# Implementation Target

Base commit: `<full-object-id>`

Ready target: [<nonempty label>](<relative-proposal-destination>)
```

The repository object format determines the full lowercase hexadecimal object
identity length. The checker accepts 40 digits for SHA-1 repositories and 64
digits for SHA-256 repositories. It rejects CRLF, an abbreviated, uppercase,
missing, duplicate, unreadable, or non-commit base, and a second raw `Base
commit:` or `Ready target:` identity line elsewhere in the file.

The destination has exactly four path segments: the parent marker `..`,
`docs`, `proposals`, and one `<name>.md` filename. The checker drops only the
parent marker and joins the remaining segments as `proposal_path`; no other dot
segment, absolute path, escape, query, entity, or link form is valid. An
optional fragment is either absent or matches
`^#[a-z0-9]+(?:-[a-z0-9]+)*$`; it becomes `proposal_anchor` without changing
`proposal_path`. The anchor must resolve to a heading in the selected proposal
blob under the repository Markdown-anchor rules.

At the declared commit, the selected page starts with one closed YAML
frontmatter block containing the exact scalar `role: proposal`. Between the
unique `## Ready` heading and the next level-two heading, the catalog contains
one column-zero list item. That item owns one link either on its first line or
on a two-space continuation line. The link destination is exactly `<name>.md`
plus the optional target anchor. Resolving that destination relative to the
catalog must equal `proposal_path` and `proposal_anchor`. An absolute
destination, escape, dot segment, code block, comment, quote, nested child
item, body text, or Blocked-section coincidence does not satisfy the catalog
check.

The declared target base must equal the candidate base. The proposal blob and
catalog blob at that commit form the normative repository snapshot. The skill
performs three semantic comparisons in this order:

1. Compare target Scope, Boundaries, and Completion Conditions with the selected
   proposal or anchored Ready subsection, including every acceptance row and
   non-goal in that selected boundary.
2. Compare the PR description and diff with that normative proposal snapshot.
3. Compare the PR description and diff with the target checklist.

An omission, addition, reversal, or out-of-scope clause on either edge is
blocking and names both conflicting clauses. The checker never reports these
semantic comparisons as passing. A target conflict invalidates the claim that
the PR implements that target; it does not override the repository proposal.
With user authorization, repository-correct work may be re-scoped under normal
Ready review. Otherwise close it and reissue a target from the intended base.

## State Model

| Target and candidate state | Result |
| --- | --- |
| One well-formed target selects a Ready proposal at the identical candidate base. | Return the seven snapshot facts and continue to semantic comparison. |
| The target base differs from the implementation branch or pull-request merge base. | Stop before implementation or acceptance and request a target reissue. |
| The target base is absent, abbreviated, duplicated, unreadable, or not a commit. | Stop and report the invalid base fact. |
| The selected path is absent, duplicated, outside `docs/proposals/`, not a proposal, or not Ready at the target base. | Stop and report the invalid selection fact. |
| Identity checks pass but the target differs from the normative proposal snapshot. | Invalidate the target-driven claim and name both conflicting clauses; do not let the target override the proposal. |
| The target matches the proposal but the PR description or diff differs from either. | Reject the PR and name both conflicting clauses. |
| No target-driven claim is present. | Do not run this preflight; normal proposal selection and review rules apply. |

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Parse one target identity. | Accept only the exact five-line identity prefix with one full object-format identity, one normalized proposal path, and an optional lowercase heading anchor. | SHA-1, SHA-256, unanchored, and anchored controls plus byte-order-mark, CRLF, missing, duplicate, abbreviated, uppercase, malformed-link, absolute-path, escaping-path, bad-anchor, query, and duplicate-identity fixtures. |
| Bind the target to repository history. | Accept only a readable commit object whose proposal and catalog blobs exist and return their full blob identities in the exact JSON schema. | Temporary histories for missing object, blob identity, tree identity, deleted proposal, missing catalog, duplicate JSON key, unknown JSON key, stdout-on-error, and non-LF output. |
| Bind the target to Ready selection. | Accept only one structurally valid `role: proposal` page and one top-level Ready link at the target commit. | Ready control plus fenced, commented, quoted, nested, Blocked, unlisted, non-proposal, duplicate-Ready, and missing-frontmatter cases. |
| Bind implementation and review to one base. | Require the branch merge base to equal the freshly advertised default tip and the pull-request merge base to equal `baseRefOid`; use that tip as the target equality candidate. | Equal control plus older-target, newer-target, default advanced without branch rebase, PR base advanced without head rebase, rebased branch, retargeted PR, missing or ambiguous remote, missing default ref, changed advertisement, missing object, shallow history, zero merge bases, and multiple merge bases. |
| Compare all three semantic edges. | Treat target-to-proposal and PR-to-proposal conflicts as blocking before using target-to-PR agreement as supporting review input. | Skill walkthroughs for exact match, same-base value-versus-identity reversal, omitted acceptance row, extra scope, misleading PR description, and repository-correct PR with an invalid target. |
| Keep repository authority separate. | Do not track target prompts or receipts, register the checker in CI, or claim that the preflight authorizes merge. | Path assertions, workflow non-registration check, ignored-target control, and skill text assertions. |
| Transfer Ready routing after completion. | Move this page to implemented records, index it, make matrix closure Ready in the catalog and its Selection State, and leave no target prompt or receipt in the range. | Exact documentation range plus missing record, missing index, catalog-only Ready, proposal-only Ready, and tracked-target mutations. |

## Completion Rule

This proposal completes when all seven acceptance rows pass, the skill is routed
by the concise `AGENTS.md` rule, and the implementation record documents the
local-only trust boundary. Move this page to implemented-proposal records,
update the implementation-record index, restore Agent Language Services
Platform Matrix Closure to Ready in both routing surfaces, and merge that PR.
Only then reissue the external matrix target from the resulting default-branch
commit.

## Non-Goals

- Tracking an external target snapshot, a JSON sidecar, or pull-request
  receipts.
- Treating target input as CI, branch-protection, or merge authorization.
- Adding a GitHub App, merge broker, ruleset, or external credential.
- Comparing arbitrary prose automatically or declaring semantic equivalence
  from hashes alone.
- Implementing the platform matrix, lifecycle inventory, MCP, LSP, plugin, or
  Veln language behavior.
