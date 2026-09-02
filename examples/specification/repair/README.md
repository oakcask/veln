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
- Broader repair-loop work needs a narrow proposal page before implementation.
- Prefer adding a case here when a repair behavior can be observed through CLI
  output, JSON records, source writes, or rollback.

## Case Routes

- `hole-preview/`: `repair --json` preview records for advisory hole
  candidates.
- `saved-preview-normalization/`: saved repair JSON input is normalized to
  command-level preview candidates.
- `apply-safe-candidate/`: `repair --apply` writes one safe candidate and
  verifies the result.
- `type-delimiter-refuse/`: `repair --apply` finds no safe candidate for
  legacy type delimiters.
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
- `source-path-casing-mixed-preview/`: repair keeps valid sibling candidates
  and excludes candidates from invalid source-path-derived module identities.
- `source-path-casing-invalid-preview/`: repair returns no candidates when the
  only candidate target is an invalid source-path-derived module identity.
- `source-path-casing-current-apply-refusal/`: current apply refuses before
  writing when source-path casing isolation leaves no safe candidate.
- `source-path-casing-saved-apply-refusal/`: saved input does not authorize a
  write to an invalid source-path-derived module identity.
- `source-path-casing-valid-preview/`: the valid sibling candidate remains
  stable when previewed without the invalid sibling.
- `partial-verification-allows-hints/`: hint-only partial verification status
  does not roll back an applied edit.

## Skip Unless Needed

- Use the route list above instead of scanning every repair case.
- Use
  [toolchain_cases](../../../crates/veln-cli/tests/toolchain_cases/) for
  low-level CLI argument, parsing, and harness edge cases where source
  readability is not part of the expected behavior.
