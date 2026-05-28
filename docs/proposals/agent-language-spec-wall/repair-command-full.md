# Repair Command Proposal Detail

Status: implemented first command boundary

This file keeps the detailed completion and remaining-work record for the
repair command. Start with [repair-command.md](repair-command.md); read this
file only when auditing the first command promotion.

## Completion Conditions

The promoted implementation resolves the first applying command boundary as
follows:

- The invocation shape is `veln repair [--json] [--apply | --dry-run]
  [--candidate CANDIDATE_ID] [path ...]`.
- Candidate input is recomputed from current source analysis; saved candidate
  files are not consumed.
- Command-level candidate records live outside diagnostic `details` in
  `repair --json` output and preserve the advisory source candidate.
- The default mode is preview. `--apply` writes exactly one safe unapplied
  single-file replacement after target validation.
- Fail-closed cases include missing safe candidates, ambiguous candidate ids,
  non-applicable selected candidates, stale spans, targets that no longer name
  holes, unsupported edit shapes, and verification failure.
- Human output and JSON output are specified in
  `../../specification/commands.md` and
  `../../specification/repair-json.md`.
- Command tests cover preview, application, refusal, JSON output, stale target
  handling, verification rollback, and preservation of the existing advisory
  `check --json` boundary.
- Remaining unresolved repair-loop behavior stays in proposal text.

## Handoff

The first applying command boundary is implemented. Use
[../../specification/repair-candidates.md](../../specification/repair-candidates.md),
[../../specification/commands.md](../../specification/commands.md), and
[../../specification/repair-json.md](../../specification/repair-json.md) for
current behavior.

Use [repair-command.md](repair-command.md) for adjacent command-level proposal
work. Do not promote broader repair-loop behavior into `../../specification/`
before implementation and tests support it.
