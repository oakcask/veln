---
role: specification
authority: normative
update-when: The `veln repair --json` output schema, candidate projection, source-path casing exclusion, verification record, or executable repair JSON evidence changes.
---

# Repair JSON

This page specifies the implemented `veln repair --json` output. Use
[repair-candidates.md](repair-candidates.md) first for the advisory candidate
boundary, [repair-application.md](repair-application.md) for apply gates, and
[commands.md](commands.md) for command availability.

## Envelope

`repair --json` emits one JSON object with:

- `schema_version`: the command output schema version.
- `tool`: object with `name` and `version`.
- `command`: always `"repair"`.
- `mode`: `"preview"` without `--apply`, or `"apply"` with `--apply`.
- `status`: `"preview"`, `"applied"`, or `"refused"`.
- `selected_candidate`: the selected command-level candidate object, or `null`.
- `candidates`: command-level repair candidates available to the current
  invocation. Current-analysis candidates whose edits target a source with a
  source-path-derived `name.invalid_case` diagnostic are excluded before
  command-local ids are assigned. When saved repair JSON inputs are present,
  this is the saved candidate set normalized for the current invocation.
- `applied_edits`: replacement edits written by the command. This is empty in
  preview and refusal output and may contain edits from more than one source
  file after a successful multi-edit candidate.
- `verification`: verification status, command, and diagnostics.
- `confirmation`: explicit user confirmation record, or `null`.
- `override`: explicit override record, or `null`.
- `summary`: candidate, applicable, applied, and refusal counts.

## Candidates

Each command-level candidate contains:

- `repair_id`: command-local id such as `repair-1`.
- `source_candidate_id`: the advisory diagnostic candidate id such as
  `symbol-1`.
- `name`: candidate symbol name when present.
- `application_policy`: copied from the advisory candidate.
- `application_status`: copied from the advisory candidate.
- `edit_summary`: human-oriented summary.
- `edits`: one or more `replace` edits with source-relative `span` and
  `replacement`.
- `verification_command`: the advisory verification command when present.
- `source`: the original advisory candidate object from diagnostic details.

Apply eligibility, selection ids, confirmation, override, target validation,
rollback, and post-edit verification are specified in
[repair-application.md](repair-application.md).

Saved command-level candidates are renumbered with current command-local
`repair_id` values. The saved command-level id remains accepted for
`--candidate` selection, but it is not emitted as a separate field. Saved
command-level candidates may use the current `edits` array shape or the legacy
single `edit` shape as input; repair JSON output emits `edits`.

## Verification

`verification.status` is:

- `"not_run"` for preview output and refusals that happen before writing.
- `"passed"` after an applied edit is written and check analysis reports no
  error diagnostics.
- `"failed"` when check analysis reports an error after writing; in this case
  every written source file is restored before output is printed.

`verification.command` is the selected candidate's advisory verification
command when available. `verification.diagnostics` uses the normal diagnostic
JSON object shape.

## Confirmation And Override

Successful output includes `confirmation` when `--confirm` was supplied:

- `confirmed_candidate_id`: the id string accepted from the command line.
- `repair_id`: the selected command-local candidate id.
- `source_candidate_id`: the selected advisory candidate id.
- `override`: `true` when `--override` was supplied.

Successful output includes `override` when `--override` was supplied:

- `application_policy`: the selected candidate policy accepted by override.
- `application_status`: the selected candidate status accepted by override.
- `accepted_obligations`: string obligations copied from the advisory
  candidate's `blocking_obligations` when present.

Preview output and refusal output set both records to `null`.

## Refusals

`status: "refused"` is stable machine-readable behavior for fail-closed cases:
missing safe candidates, missing or ambiguous requested candidate ids,
non-applicable selected candidates, missing or mismatched confirmation,
saved candidates that are not current, stale target spans, targets that no
longer name holes, overlapping edits,
verification failure, and unsupported edit shapes.

`summary.refusal_reason` carries a short stable-enough routing string for human
and agent workflows. It is not a diagnostic id.
