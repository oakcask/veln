# Repair Command Completion Review

Status: complete for the promoted repair command boundary.

This review covers
[../proposals/agent-language-spec-wall/repair-command.md](../proposals/agent-language-spec-wall/repair-command.md).
Current behavior is specified in
[../specification/commands.md](../specification/commands.md),
[../specification/repair-candidates.md](../specification/repair-candidates.md),
and [../specification/repair-json.md](../specification/repair-json.md).

## Completion Check

- `veln repair [--json] [--apply | --dry-run] [--candidate CANDIDATE_ID]
  [path ...]` is implemented as a dedicated command.
- Candidate input is recomputed from current source analysis; saved candidate
  files are still out of scope.
- `repair --json` emits command-level candidates outside diagnostic `details`
  while preserving the original advisory candidate as `source`.
- Preview is the default mode and writes no files.
- `repair --apply` applies exactly one safe unapplied single-file replacement
  after target validation and post-edit check verification.
- Refusal paths cover no safe candidates, ambiguous candidate selection,
  non-applicable selected candidates, stale spans, non-hole targets, unsupported
  edit shapes, and verification failure.
- Existing `check --json` advisory candidates remain unapplied, and `check`
  still rejects repair-application flags.

## Remaining Scope

Saved candidate files, multi-file or multi-span repairs, confirmation,
override, partial application, and external verification commands remain
proposal work. They should not be treated as current behavior until a later
specification page states them.

## Verification

- `cargo test -p veln-cli check_json -- --nocapture`
- `cargo test -p veln-cli commands::repair -- --nocapture`
- `cargo test -p veln-cli cli::tests -- --nocapture`
