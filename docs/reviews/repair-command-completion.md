# Repair Command Completion Review

Status: complete for the promoted repair command, saved candidate input, and
multi-edit apply boundaries.

This review covers
[../proposals/agent-language-spec-wall/repair-command.md](../proposals/agent-language-spec-wall/repair-command.md).
Current behavior is specified in
[../specification/commands.md](../specification/commands.md),
[../specification/repair-candidates.md](../specification/repair-candidates.md),
and [../specification/repair-json.md](../specification/repair-json.md).

## Completion Check

- `veln repair [--json] [--apply | --dry-run] [--candidate CANDIDATE_ID]
  [path ...]` is implemented as a dedicated command.
- Candidate input can come from current source analysis or saved repair JSON
  input. Saved input may be repair JSON, command-level candidates, check JSON,
  or advisory candidates.
- `repair --json` emits command-level candidates outside diagnostic `details`
  while preserving the original advisory candidate as `source`; saved
  command-level ids remain selectable but are renumbered for output.
- Preview is the default mode and writes no files.
- `repair --apply` applies exactly one safe unapplied candidate after target
  validation and post-edit check verification. The selected candidate may carry
  multiple replacement edits in one source file or across multiple source
  files.
- Saved candidate input is not a write authorization. Apply requires current
  safe evidence for non-empty replacement edits before writing, and explicit
  empty `satisfy` suffix removals are constrained to the current source text.
- Refusal paths cover no safe candidates, missing or ambiguous candidate
  selection, non-applicable selected candidates, saved candidates that are not
  current, stale spans, non-hole targets, overlapping edits, unsupported edit
  shapes, and verification failure.
- Existing `check --json` advisory candidates remain unapplied, and `check`
  still rejects repair-application flags.

## Remaining Scope

Confirmation, override, partial application, and external verification commands
remain proposal work. They should not be treated as current behavior until a
later specification page states them.

## Verification

- `cargo test -p veln-cli repair_ --test check_json`
- `cargo test -p veln-cli repair --bins`
- `cargo test -p veln-cli repair`
- `cargo test -p veln-cli cli_prints_help_for_empty_invocation_and_subcommand_help --test check_json`
- `cargo test -p veln-diagnostics parses_json_values`
- `cargo test -p veln-diagnostics parse_json_value`
