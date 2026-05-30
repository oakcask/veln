# Repair Application

This page specifies the implemented `veln repair --apply` write boundary. Use
[repair-candidates.md](repair-candidates.md) first when the task is about
candidate fields, ranking, or advisory `check --json` behavior.

## Current Boundary

- `veln repair --apply` is fail-closed and applies exactly one selected
  candidate.
- Safe automatic application requires
  `application_policy: "safe_repair_candidate"` and
  `application_status: "unapplied"`.
- Saved repair JSON input is not write authorization. Without override, every
  non-empty saved replacement edit must match current safe evidence with the
  same `source_candidate_id`, application policy, and application status.
- Explicit empty `satisfy` suffix removals are validated against the current
  source text.
- Partial application of a candidate's replacement set is not implemented.

## Selection

When `--candidate` is present, selection may use the command-local `repair_id`,
the preserved advisory `source_candidate_id`, or a saved command-level id. A
missing or ambiguous id refuses before writing.

When no candidate id is supplied, application selects the only safe unapplied
candidate. If there are none, or if more than one safe unapplied candidate is
available, application refuses before writing.

## Target Gates

Every selected replacement target must pass these checks before and during the
write:

- The target file path is source-relative.
- The target span is still in bounds and on character boundaries.
- Non-empty replacement targets still name holes.
- Explicit empty replacements are limited to current `satisfy` suffix removal.
- Edits in the same file do not overlap.

For a single hole replacement with no explicit suffix-removal edit, applying
the repair also replaces the hole's `satisfy` suffix with the candidate
replacement.

## Confirmation And Override

`--confirm CANDIDATE_ID` records explicit user confirmation for the selected
candidate. When `--candidate` is also present, both ids must resolve to the
same candidate; otherwise application refuses before writing.

`--override` requires `--confirm` and permits a selected
`manual_review_required` candidate to pass the application-policy gate. The
selected candidate must still be `unapplied`, and every replacement target must
pass the same source-relative, current-span, hole-target, explicit empty
replacement, non-overlap, rollback, and post-edit verification rules as the
safe path.

Override may accept saved candidate input without matching current safe
evidence, but only after those target and verification gates pass.

## Verification And Rollback

After writing, `repair --apply` reruns the same check analysis over the
selected inputs. If verification reports any error diagnostic, the command
restores the original contents of every written file and exits unsuccessfully.

Hint-only partial status, including remaining holes elsewhere, does not by
itself roll back the edit.

Successful JSON output records `confirmation` and, when override was used,
`override`. Refusals do not write files and leave those records null.

## Remaining Proposal Boundary

Do not promote any remaining repair-loop proposal axis into this specification
until the behavior is implemented and tested. The current proposal route for
verification orchestration, ranking evidence, edit granularity, and broader
application authority is
[../proposals/agent-repair-loop-followups.md](../proposals/agent-repair-loop-followups.md).
