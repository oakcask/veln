---
role: proposal
update-when: Repair candidate extraction, source-path-derived module identity isolation, repair preview or apply selection, or the planned isolation evidence changes.
---

# Identifier Casing Repair Candidate Isolation

## Summary

Exclude repair candidates whose edits target a source with an invalid
source-path-derived module identity. Continue to return candidates from
unrelated valid sources selected by the same `veln repair` invocation.

This is a bounded extension of the implemented source-path casing and repair
contracts. It does not add a source syntax, name class, command, or transport
surface.

## Implemented Prerequisites

- [Name Resolution](../specification/name-resolution.md) specifies that an
  invalid source-path-derived identity is not registered as a normal module.
- [Repair Command](../specification/command-repair.md) specifies that `repair`
  uses shared project analysis and extracts advisory candidates from current
  diagnostics.
- [Repair JSON](../specification/repair-json.md) specifies current and saved
  candidate selection, preview output, application refusal, and verification.

`repair` is a diagnostic-tolerant consumer. The checked
`../../examples/specification/repair/discovery-parse-gate/` case shows that one
rejected source does not hide candidates from another parse-clean source. This
proposal preserves that tolerant boundary for source-path casing.

## Candidate Selection Contract

A current-analysis repair candidate is ineligible when any candidate edit
targets a source whose package-relative path reports a source-path-derived
`name.invalid_case` diagnostic. The candidate does not appear in preview
output and cannot be selected for application.

Candidates whose edits target only sources with valid derived module
identities remain eligible. Filtering happens before command-local
`repair-N` identifiers and summary counts are assigned, so output contains no
identifier gap caused by an excluded candidate.

Saved candidate input does not bypass the boundary. An excluded candidate
cannot satisfy the current-candidate match required for application. Previewing
saved input remains an inspection operation and does not authorize an edit.

The source-path casing diagnostic remains available through `veln check` and
other diagnostic-producing commands. `repair` does not add a second casing
diagnostic to its candidate envelope.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Preview one invalid-path source that contains a safe hole candidate and one valid-path source that contains a different safe candidate. | Return only the valid source candidate as `repair-1`; set `candidate_count` and `applicable_count` to one; emit no edit whose file is the invalid path. | One checked `repair --json` mixed-source case. |
| Preview only the invalid-path source. | Return an empty candidate list and zero candidate counts. | A second command invocation in the mixed-source case or one focused checked case. |
| Apply a current or saved candidate whose edit targets the invalid-path source. | Refuse before writing and leave every selected source unchanged. | Checked JSON refusal and file-equality assertions. |
| Preview the same valid-path source without the invalid sibling. | Return the same candidate target, replacement, and source candidate evidence as the mixed-source invocation. | Response-local assertions in the mixed-source case. |

## Completion

The proposal is complete when all four rows pass, the repair specification
states the source-path candidate isolation rule, and executable repair evidence
is routed from the specification. Move the completed record to
`../reference/implemented-proposals/` and remove this page from the proposal
catalog.
