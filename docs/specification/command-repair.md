---
role: specification
authority: normative
update-when: The veln repair command preview, apply, confirmation, verification, rollback, or JSON behavior changes.
---

# Repair Command

`repair` uses the same source discovery and static analysis path as `check` to
collect advisory hole repair candidates. Without `--apply`, the command is a
preview: it prints command-level repair candidates and writes no source files.
`--dry-run` is an explicit spelling of that default preview mode.

Candidate input is recomputed from the current source files unless one or more
`*.json` inputs are present. A JSON input is treated as saved repair candidate
input, not as a source file. Saved input may be a `repair --json` envelope, a
command-level candidate object or array, a `check --json` envelope, or an
advisory candidate object or array. Current-analysis candidate filtering is
specified in [repair-candidates.md](repair-candidates.md) and projected into
[repair-json.md](repair-json.md). Command-level candidate ids use the form
`repair-N` and are assigned for the current invocation. The original advisory
candidate id from diagnostic details is also preserved as
`source_candidate_id`. `--candidate` may name either id, or a saved
command-level id from a saved repair candidate, but application refuses
ambiguous ids.

Application is deliberately narrow. `--apply` applies exactly one selected
candidate; saved candidate input remains advisory rather than write
authorization. Selection, safe application, confirmation, override, target
validation, partial-application non-support, post-edit verification, and
rollback are specified in
[repair-application.md](repair-application.md).

Human preview output lists candidate ids, summaries, a representative target
span, replacement, and application policy. Human apply output reports the
applied candidate and verification result. Human refusal output starts with
`repair refused:` followed by the failed gate.

With `--json`, `repair` emits the repair JSON record described in
[repair-json.md](repair-json.md).
