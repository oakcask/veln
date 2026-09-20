---
role: specification
authority: normative
update-when: The veln repair command preview, apply, confirmation, verification, rollback, or JSON behavior changes.
specification-coverage: usage=#repair-command; behavior=#apply-boundary; limits=#limits-and-errors
---

# Repair Command

Use `veln repair [PATH ...]` to collect advisory hole-repair candidates.
The default mode and explicit `--dry-run` are previews; neither writes source
files. Use `--json` for the machine record in [repair-json.md](repair-json.md).

## Candidate input and selection

Candidates are recomputed from current source unless an input path ends in
`.json`. A JSON input may be a repair envelope, command-level candidate,
candidate array, `check --json` envelope, or advisory candidate object/array.
Saved candidates remain advisory. Current candidates whose target source has a
source-path-derived `name.invalid_case` are excluded. Current command ids are
`repair-N`; the preserved advisory id is `source_candidate_id`.

Use `--candidate ID` to select a command-local, advisory, or accepted saved
command-level id. A missing or ambiguous id refuses. Without an id, apply mode
requires exactly one safe unapplied candidate.

## Apply boundary

Use `--apply` to apply exactly one candidate. Safe application requires
`application_policy: "safe_repair_candidate"` and
`application_status: "unapplied"`. Use `--confirm ID` to record explicit
confirmation. `--override` requires `--confirm` and permits a
`manual_review_required` candidate, but does not bypass target, overlap,
freshness, or verification checks. Partial application is unsupported.

After writing, the command reruns check analysis over the selected inputs. Any
error diagnostic restores every written file and fails. Hint-only status and
remaining unrelated holes do not roll back a successful edit. Refusals do not
write files.

## Limits and errors

Saved JSON never authorizes a write by itself. Targets must remain
source-relative, in bounds, on character boundaries, and non-overlapping;
non-empty targets must still name holes. Explicit empty replacements are only
current `satisfy` suffix removals. A single hole replacement also replaces
that hole's suffix unless an explicit suffix-removal edit is supplied.

## References

Implementation: `crates/veln-cli/src/commands/repair.rs`. Application gates
and rollback are specified in [repair-application.md](repair-application.md);
candidate fields are specified in [repair-candidates.md](repair-candidates.md).
