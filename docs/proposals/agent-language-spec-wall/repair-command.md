# Repair Command Proposal Route

Status: implemented

This page records the promoted repair-command proposal area without requiring
the broad design brief or full open-question inventory first.

## Read First

- Current advisory repair candidate behavior, application-policy boundary, and
  command gate:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Implemented command behavior:
  [../../specification/commands.md](../../specification/commands.md).
- Implemented repair JSON output:
  [../../specification/repair-json.md](../../specification/repair-json.md).
- Safe repair candidate boundary:
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md).
- Completion review for this command promotion:
  [../../reviews/repair-command-completion.md](../../reviews/repair-command-completion.md).
- Earlier advisory candidate boundary review:
  [../../reviews/agent-language-spec-wall-completion.md](../../reviews/agent-language-spec-wall-completion.md).

## Implemented Boundary

The implemented boundary is
[advisory repair candidates](../../specification/repair-candidates.md) plus a
narrow `veln repair` command gate. `repair` previews command-level candidate
records and `repair --apply` can apply exactly one safe unapplied advisory
candidate after post-edit check verification.

## Remaining Command-Level Work

- Saved candidate files or other inputs beyond recomputing from source.
- Multi-file or multi-span repairs.
- Confirmation and override protocol.
- Verification commands beyond the built-in post-edit check analysis.
- Broader ranking models and evidence payloads beyond the advisory candidate
  source preserved in repair JSON.

## Command Detail

Use [repair-command-full.md](repair-command-full.md) only when auditing the
implemented completion record or planning the remaining command-level work.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Changing implemented repair command behavior.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `explain`, or a future command.

## Skip Unless Needed

- Use [open-questions.md](open-questions.md) only when auditing the historical
  design-wall inventory.
- Use [design-brief.md](design-brief.md) only when the broad repair-loop thesis
  is needed.
- Use [repair-command-full.md](repair-command-full.md) only when auditing the
  completed command criteria or planning the remaining command-level work.
