# Repair Candidates

This page is the entry point for the implemented boundary around advisory
repair candidates. Start here when a task mentions repair candidates, safe
repair, candidate edits, applying edits, or the repair command.

## Current Boundary

- Candidate records may appear in `veln check --json` diagnostics. They are
  advisory records, not an applying workflow.
- Candidate edits are concrete replacement suggestions for the reported span,
  but command execution leaves them unapplied.
- Candidate application policy is evidence and review routing. Even
  `safe_repair_candidate` means the implemented static subset has discharged;
  it authorizes only the narrow `veln repair --apply` gate described below.
- `veln repair` previews command-level repair records and can apply one safe
  unapplied advisory candidate after rerunning check analysis. Confirmation,
  override, saved candidate files, multi-file edits, and partial application
  remain outside the implemented boundary.
- The command recomputes candidate input from current source analysis. It does
  not consume saved candidate files.

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
  advisory candidate id is preserved as `source_candidate_id`.
- `verification_hint` names the check to run after a human or future command
  applies an edit.

## Choose Detail

- Candidate fields, stable `check --json` envelope, diagnostic spans, and
  `details` payload boundaries: [diagnostics-json.md](diagnostics-json.md).
- Candidate ranking, `satisfy` repair constraints, safe-repair matching, and
  exact examples: [holes.md](holes.md).
- Implemented command availability and command gates: [commands.md](commands.md).
- `repair --json` output: [repair-json.md](repair-json.md).
- Rationale for keeping advisory candidates separate from edit application:
  [source-decisions.md](source-decisions.md).
- Remaining proposal material for confirmation, override, and broader applying
  workflows:
  [../proposals/agent-language-spec-wall/repair-command.md](../proposals/agent-language-spec-wall/repair-command.md).

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
and `application_status: "unapplied"`, the target is a single source-relative
replacement, the span still names a hole in the current file, and post-edit
check analysis reports no error diagnostics.

If verification fails after writing, the original source file is restored.
Hint-only partial status, including remaining holes elsewhere, does not by
itself roll back the edit.

## Remaining Proposal Boundary

Do not promote confirmation, override, saved candidate files, multi-file edit
application, partial application, or broader automatic repair behavior into
this specification until the behavior is implemented and tested. Until then,
keep that material in
[../proposals/agent-language-spec-wall/repair-command.md](../proposals/agent-language-spec-wall/repair-command.md).
