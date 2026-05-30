# Repair Command Confirmation And Override

Status: implemented

This record keeps completion evidence for the implemented `veln repair`
confirmation and override target. Use the specification pages for current
behavior.

## Read First

- Current repair candidate behavior:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Current command behavior:
  [../../specification/commands.md](../../specification/commands.md).
- Current repair JSON behavior:
  [../../specification/repair-json.md](../../specification/repair-json.md).
- Remaining proposal route:
  [../../proposals/agent-repair-loop-followups.md](../../proposals/agent-repair-loop-followups.md).

## Outcome

The completed target added explicit user confirmation and manual-review
override recording around repair application.

- `--confirm CANDIDATE_ID` records the id the user confirmed before writing.
- `--override` requires `--confirm` and can apply one confirmed
  `manual_review_required` candidate.
- Override records the accepted application policy, status, and advisory
  blocking obligations in repair JSON.
- Override keeps the normal source-relative target, stale-span, hole-target,
  overlap, rollback, and post-edit verification gates.

## Boundary

This target did not add partial application, broader ranking models, external
verification commands, or general automatic repair behavior. Those remain
proposal work until a short proposal page selects one concrete target.
