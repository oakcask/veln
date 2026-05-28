# Repair JSON

This page specifies the implemented `veln repair --json` output. Use
[repair-candidates.md](repair-candidates.md) first for the advisory candidate
boundary and [commands.md](commands.md) for command gates.

## Envelope

`repair --json` emits one JSON object with:

- `schema_version`: the command output schema version.
- `tool`: object with `name` and `version`.
- `command`: always `"repair"`.
- `mode`: `"preview"` without `--apply`, or `"apply"` with `--apply`.
- `status`: `"preview"`, `"applied"`, or `"refused"`.
- `selected_candidate`: the selected command-level candidate object, or `null`.
- `candidates`: all command-level repair candidates found in the current
  invocation. When saved repair JSON inputs are present, this is the saved
  candidate set normalized for the current invocation.
- `applied_edits`: replacement edits written by the command. This is empty in
  preview and refusal output.
- `verification`: verification status, command, and diagnostics.
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
- `edit`: a single `replace` edit with source-relative `span` and
  `replacement`.
- `verification_command`: the advisory verification command when present.
- `source`: the original advisory candidate object from diagnostic details.

`repair --apply` applies only candidates whose `application_policy` is
`"safe_repair_candidate"` and whose `application_status` is `"unapplied"`.
When `--candidate` is present, the requested id may match either `repair_id` or
`source_candidate_id`.

Saved command-level candidates are renumbered with current command-local
`repair_id` values. The saved command-level id remains accepted for
`--candidate` selection, but it is not emitted as a separate field.

## Verification

`verification.status` is:

- `"not_run"` for preview output and refusals that happen before writing.
- `"passed"` after an applied edit is written and check analysis reports no
  error diagnostics.
- `"failed"` when check analysis reports an error after writing; in this case
  the source file is restored before output is printed.

`verification.command` is the selected candidate's advisory verification
command when available. `verification.diagnostics` uses the normal diagnostic
JSON object shape.

## Refusals

`status: "refused"` is stable machine-readable behavior for fail-closed cases:
missing safe candidates, missing or ambiguous requested candidate ids,
non-applicable selected candidates, saved candidates that are not current,
stale target spans, targets that no longer name holes, verification failure,
and unsupported edit shapes.

`summary.refusal_reason` carries a short stable-enough routing string for human
and agent workflows. It is not a diagnostic id.
