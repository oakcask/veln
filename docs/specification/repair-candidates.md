---
role: specification
authority: normative
update-when: The advisory repair candidate fields, application policy, command inputs, or repair JSON behavior changes.
specification-coverage: usage=#input-route; behavior=#current-boundary; limits=#limits
---

# Repair Candidates

Repair candidates are advisory replacement records. `veln check --json`
reports them inside `candidate_queries`; checking never edits source. A query
may contain ranked hole-fill candidates or concrete parse-repair edits. Each
candidate remains unapplied and carries its source span, replacement, review
policy, verification hint, known limits, and blocking obligations.

## Current Boundary

`safe_repair_candidate` means that the implemented static matcher discharged
the candidate's proof obligation. It is eligible for the ordinary apply gate,
but it is still advisory until `veln repair` selects it and verification
completes. Other candidates use `manual_review_required`. `application_status`
is initially `"unapplied"`; `application_policy` never grants write access by
itself.

The command assigns local `repair-N` identifiers while preserving the original
diagnostic candidate id as `source_candidate_id`. `--candidate` may select
either id; ambiguous matches fail. A selected command candidate may contain
multiple replacement edits across multiple source files.

Candidates whose edits target a source with a source-path-derived
`name.invalid_case` diagnostic are excluded before command ids are assigned.
Valid sibling sources remain eligible. Legacy type delimiter spellings are not
repair candidate classes.

## Input Route

With source inputs, `veln repair` reruns analysis, normalizes the advisory
records into command candidates, and optionally selects one. If one or more
JSON inputs are supplied, the displayed set comes from a repair envelope,
command candidate object or array, `check --json` envelope, or advisory
candidate object or array. Source inputs still determine project discovery and
post-edit verification. Saved input is selection data; it never authorizes a
write and is renumbered for the current invocation, although its saved command
id remains selectable.

## Preview and Apply

`veln repair` previews the normalized candidates and their edits. Applying a
safe candidate requires an unapplied record, a matching current analysis, a
valid target shape and source span, no overlapping edits, and successful
post-edit verification with rollback on failure. `verification_hint` describes
the check to run after the edit, and `--confirm` records the id selected by the
user.

`--override --confirm CANDIDATE_ID` permits one explicitly confirmed
`manual_review_required` candidate and records the override, policy, status,
and blocking obligations. Override applies only to the review policy: it never
bypasses stale-input, target-shape, source-path, span, overlap, rollback, or
post-edit verification gates. Saved or ambiguous candidates therefore remain
unapplied until those independent checks pass.

## Limits

Candidate generation is bounded by the current analysis result. Applying one
candidate always rechecks current source identity, target spans, edit overlap,
confirmation, rollback, and verification. A failed check leaves the source
unchanged. Candidate ranking and `satisfy` matching are specified in
[holes.md](holes.md); field and envelope schemas are specified in
[diagnostics-json.md](diagnostics-json.md) and [repair-json.md](repair-json.md).

## References

- Application validation and rollback: [repair-application.md](repair-application.md).
- Command availability: [commands.md](commands.md).
- Advisory/application rationale: [source-decisions.md](source-decisions.md).
