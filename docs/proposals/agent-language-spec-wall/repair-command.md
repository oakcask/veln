# Repair Command Proposal Route

Status: open-proposal

This page routes the repair-loop proposal area without requiring the broad
design brief or full open-question inventory first.

## Read First

- Implemented command behavior:
  [../../reference/language/commands.md](../../reference/language/commands.md).
- Implemented hole repair candidate records:
  [../../reference/language/holes.md](../../reference/language/holes.md).
- Implemented `check --json` diagnostic shape:
  [../../reference/language/diagnostics-json.md](../../reference/language/diagnostics-json.md).
- Safe repair candidate boundary:
  [../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../../reference/source-decisions/records/result-safe-repair-candidate-boundary.md).

## Current Boundary

- No dedicated `repair` command is implemented.
- `check --json` may expose ranked hole candidates and concrete replacement
  edits, but every emitted edit remains unapplied.
- `safe_repair_candidate` means the candidate has discharged the implemented
  static repair subset; it does not authorize automatic application.

## Open Command-Level Work

- Final command name and invocation shape.
- Candidate schema outside the current diagnostic `details` payload.
- Edit representation for multi-file or multi-span repairs.
- Ranking model and evidence payload.
- Confirmation, override, and verification protocol.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Promoting repair command behavior into current implementation.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `explain`, or a future command.

## Skip Unless Needed

- Use [open-questions.md](open-questions.md) only when auditing the historical
  design-wall inventory.
- Use [design-brief.md](design-brief.md) only when the broad repair-loop thesis
  is needed.
