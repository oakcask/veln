# Repair Candidates

This page is the entry point for the implemented boundary around advisory
repair candidates. Start here when a task mentions repair candidates, safe
repair, candidate edits, applying edits, or the repair command.

## Current Boundary

- Candidate records may appear in `veln check --json` diagnostics. They are
  advisory records, not an applying workflow.
- Candidate edits are concrete replacement suggestions tied to source spans,
  but `check` command execution leaves them unapplied.
- Candidate application policy is evidence and review routing. Even
  `safe_repair_candidate` means the implemented static subset has discharged;
  it authorizes only the narrow `veln repair --apply` gate described below.
- `veln repair` previews command-level repair records and can apply one safe
  unapplied advisory candidate after rerunning check analysis. One selected
  candidate may contain multiple replacement edits and may touch more than one
  source file.
- `veln repair --apply --override --confirm CANDIDATE_ID` can apply one
  explicitly confirmed manual-review candidate while recording the override.
  The override path does not skip target-shape, stale-span, overlap,
  rollback, or post-edit verification gates.
- The command can load candidate input from current source analysis or saved
  repair JSON files. Saved inputs do not authorize writes by themselves; apply
  still requires either a matching current safe candidate or explicit override
  confirmation.

## Concept Map

- `candidate_queries` are diagnostic `details` records that describe how to
  look for hole fills.
- `candidates` are ranked, source-backed replacement suggestions inside a
  query.
- `application_policy` describes review and evidence state. It is not a write
  authorization.
- `application_status: "unapplied"` is the current behavior for emitted
  candidate edits.
- `repair_id` is the command-local id emitted by `veln repair`; the original
  advisory candidate id is preserved as `source_candidate_id`. `--candidate`
  can match either id, and ambiguous matches refuse.
- Candidate selection chooses one command-level candidate. Candidate input
  decides where the displayed command-level candidates come from.
- Saved command-level repair JSON input is renumbered for the current
  invocation, but selection may also match the saved command-level id.
- `verification_hint` names the check to run after a human or command applies
  candidate edits.
- `--confirm` records the user-confirmed id that resolved to the selected
  candidate.
- `--override` accepts a non-safe manual-review policy only with explicit
  confirmation and records the accepted policy, status, and advisory blocking
  obligations.

## Input Route

- Source input route: read source inputs, rerun analysis, normalize advisory
  candidates into command-level repair candidates, then optionally select one
  by id.
- Saved input route: when one or more `*.json` inputs are present, load
  candidates from saved repair JSON instead of using recomputed candidates as
  the displayed candidate set. Source inputs still control project discovery
  and verification.
- Saved input may be a `repair --json` envelope, a command-level candidate
  object or array, a `check --json` envelope, or an advisory candidate object or
  array.

## Choose Detail

- Candidate fields, stable `check --json` envelope, diagnostic spans, and
  `details` payload boundaries: [diagnostics-json.md](diagnostics-json.md).
- Candidate ranking, `satisfy` repair constraints, safe-repair matching, and
  exact examples: [holes.md](holes.md).
- Implemented command availability and command gates: [commands.md](commands.md).
- `repair --json` output: [repair-json.md](repair-json.md).
- Rationale for keeping advisory candidates separate from edit application:
  [source-decisions.md](source-decisions.md).
- Proposal route for broader applying workflows:
  [../proposals/agent-repair-loop-followups.md](../proposals/agent-repair-loop-followups.md).

## Read When

- Changing candidate record fields, ranking, edits, evidence, known limits,
  blocking obligations, verification hints, or application policy.
- Deciding whether repair-loop behavior belongs in implemented `check --json`
  diagnostics, implemented `repair`, or proposal work.
- Auditing that proposal text stays subordinate to current implemented
  behavior.

## Skip Unless Needed

- Use [holes-full.md](holes-full.md) only for exact candidate examples or
  matching rules.
- Use [diagnostics-json-full.md](diagnostics-json-full.md) only for the full
  diagnostic field catalog.
- Open
  [../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  only when the advisory-versus-application rationale is needed.

## Apply Rule

`veln repair --apply` is fail-closed. It applies exactly one candidate only when
the selected advisory candidate has `application_policy: "safe_repair_candidate"`
and `application_status: "unapplied"`, every replacement target is
source-relative and still valid in the current file, non-empty replacements
still name holes, explicit empty replacements are limited to `satisfy` suffix
removal, edits in the same file do not overlap, and post-edit check analysis
reports no error diagnostics.

When `--candidate` is present, selection may use either the command-local
`repair_id` or the preserved advisory `source_candidate_id`. A missing or
ambiguous id refuses before writing.

For saved candidate input, apply also requires current safe evidence for each
non-empty replacement edit with the same `source_candidate_id`, application
policy, and application status. Explicit empty `satisfy` suffix removals are
validated against the current source text. A saved candidate that is stale,
unsupported, or no longer current refuses before writing.

If verification fails after writing, every source file written by the candidate
is restored. Hint-only partial status, including remaining holes elsewhere,
does not by itself roll back the edit.

## Confirmation And Override

`--confirm CANDIDATE_ID` records explicit user confirmation for the selected
candidate. When `--candidate` is also present, both ids must resolve to the same
candidate; otherwise application refuses before writing.

`--override` requires `--confirm` and permits a selected
`manual_review_required` candidate to pass the application-policy gate. The
selected candidate must still be `unapplied`, and every replacement target must
pass the same source-relative, current-span, hole-target, explicit empty
replacement, non-overlap, rollback, and post-edit verification rules as the
safe path. Override may accept saved candidate input without matching current
safe evidence, but only after those target and verification gates pass.

Successful JSON output records `confirmation` and, when override was used,
`override`. Refusals do not write files and leave those records null.

## Remaining Proposal Boundary

Do not promote any remaining repair-loop proposal axis into this specification
until the behavior is implemented and tested. The current proposal route for
verification orchestration, ranking evidence, edit granularity, and broader
application authority is
[../proposals/agent-repair-loop-followups.md](../proposals/agent-repair-loop-followups.md).
