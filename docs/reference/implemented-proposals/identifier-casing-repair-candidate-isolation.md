---
role: implementation-record
authority: supporting
update-when: Identifier casing repair candidate isolation, repair command specifications, or executable repair source-path casing examples change.
---

# Identifier Casing Repair Candidate Isolation

## Outcome

`veln repair` excludes current-analysis candidates whose edits target a source
with a source-path-derived `name.invalid_case` diagnostic. The command keeps
eligible candidates from valid sibling sources and assigns command-local
`repair-N` ids after the exclusion.

Saved repair JSON input remains advisory. A saved candidate that targets an
invalid source-path-derived module identity cannot satisfy the current safe
candidate match required for ordinary application, so refusal leaves selected
source files unchanged.

Current behavior is specified by
[Repair Candidates](../../specification/repair-candidates.md),
[Repair Application](../../specification/repair-application.md), and
[Repair JSON](../../specification/repair-json.md).

## Evidence

| Case | Required result | Evidence |
| --- | --- | --- |
| Preview an invalid-path source with a safe hole candidate and a valid sibling source with a different safe candidate. | Return only the valid source candidate as `repair-1`; set `candidate_count` and `applicable_count` to one; emit no edit whose file is the invalid path. | `examples/specification/repair/source-path-casing-mixed-preview/` |
| Preview only the invalid-path source. | Return an empty command-level candidate list and zero candidate counts. | `examples/specification/repair/source-path-casing-invalid-preview/` |
| Apply a current candidate whose edit targets the invalid-path source. | Refuse before writing and leave the selected source unchanged. | `examples/specification/repair/source-path-casing-current-apply-refusal/` |
| Apply a saved candidate whose edit targets the invalid-path source. | Refuse before writing because the saved candidate is not current, and leave the selected source unchanged. | `examples/specification/repair/source-path-casing-saved-apply-refusal/` |
| Preview the same valid-path source without the invalid sibling. | Return the same valid candidate target and replacement as the mixed-source invocation. | `examples/specification/repair/source-path-casing-valid-preview/` |

The CLI integration tests also compare the valid candidate's command edits and
source candidate evidence between mixed-source and valid-only previews.

## Scope

This record completes only the repair candidate and ordinary apply boundary
for source-path-derived module identity casing. It does not add explicit import
aliases, MCP rename mapping, new diagnostics, source syntax, or broader repair
policy.
