# Repair Specification Cases

These executable cases cover the public `veln repair` preview and apply
surfaces. Start here when changing repair candidate normalization, write
authorization, confirmation, override, verification, rollback, or saved JSON
input behavior.

## Read First

- Current behavior is specified in
  [repair-candidates.md](../../../docs/specification/repair-candidates.md),
  [repair-application.md](../../../docs/specification/repair-application.md),
  and [repair-json.md](../../../docs/specification/repair-json.md).
- Remaining repair-loop proposal work is routed through
  [agent-repair-loop-followups.md](../../../docs/proposals/agent-repair-loop-followups.md).
- Prefer adding a case here when a repair behavior can be observed through CLI
  output, JSON records, source writes, or rollback.

## Case Routes

- `hole-preview/`: `repair --json` preview records for advisory hole
  candidates.
- `saved-preview-normalization/`: saved repair JSON input is normalized to
  command-level preview candidates.
- `apply-safe-candidate/`: `repair --apply` writes one safe candidate and
  verifies the result.
- `apply-confirmed-override/`: confirmed override applies and records a
  manual-review candidate.
- `refuse-multiple-candidates/`: automatic apply refuses ambiguous safe
  candidates until one is selected.
- `refuse-override-without-confirm/`: override application requires explicit
  confirmation.
- `saved-apply-requires-current-match/`: saved repair JSON does not authorize
  writes without a current safe candidate match.
- `verification-checked-core-rollback/`: repair verification rolls back when
  shared check analysis reports a checked-core blocker.
- `discovery-parse-gate/`: repair follows the shared discovery and parse-clean
  analysis gate.
- `partial-verification-allows-hints/`: hint-only partial verification status
  does not roll back an applied edit.

## Skip Unless Needed

- Use the route list above instead of scanning every repair case.
- Use
  [toolchain_cases](../../../crates/veln-cli/tests/toolchain_cases/) for
  low-level CLI argument, parsing, and harness edge cases where source
  readability is not part of the expected behavior.
