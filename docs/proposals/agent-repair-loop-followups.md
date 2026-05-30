# Agent Repair Loop Follow-Ups

Status: proposed

This page routes the repair-loop axes that remain beyond the implemented
advisory candidate and narrow applying command boundary. Proposal text here is
not current repair behavior unless `../specification/` also states it.

## Read First

- Current advisory candidates:
  [../specification/repair-candidates.md](../specification/repair-candidates.md).
- Current command behavior:
  [../specification/commands.md](../specification/commands.md).
- Current applying-command gates:
  [../specification/repair-application.md](../specification/repair-application.md).
- Current repair JSON:
  [../specification/repair-json.md](../specification/repair-json.md).
- Completed command records:
  [../reference/implemented-proposals/repair-command-first-boundary.md](../reference/implemented-proposals/repair-command-first-boundary.md)
  and
  [../reference/implemented-proposals/repair-command-confirmation-override.md](../reference/implemented-proposals/repair-command-confirmation-override.md).

## Current Base

The implemented base is advisory repair candidates in `check --json`, plus the
narrow `veln repair` preview and apply gate documented in
`../specification/`. Do not restate those gates here when the specification page
already covers them.

## Proposal Axes

- Verification orchestration: external verification commands, selected tests,
  or a command mode that coordinates repair and verification without treating
  passing tests alone as proof of correctness.
- Candidate evidence: broader ranking models or evidence payloads beyond the
  advisory candidate source preserved in repair JSON.
- Edit granularity: partial application of a candidate's replacement set.
- Application authority: general automatic repair behavior beyond the current
  explicit safe or confirmed override gates.

Each axis needs a concrete target before implementation. Keep cross-axis
coordination here, but split large design work into a narrower proposal page
when the first implementation target is selected.

## Code Route

- Command entry point: `crates/veln-cli/src/commands/repair.rs`.
- Candidate input normalization and id matching:
  `crates/veln-cli/src/commands/repair/candidates.rs`.
- Application authority, confirmation, override, and verification:
  `crates/veln-cli/src/commands/repair/application.rs`.
- Edit planning and target validation:
  `crates/veln-cli/src/commands/repair/editing.rs`.
- Repair command JSON and human output:
  `crates/veln-cli/src/commands/repair/outcome.rs`.
- Advisory candidate evidence construction:
  `crates/veln-sema/src/repair_candidates.rs`.

## Non-Targets

- Do not weaken the current advisory boundary for `check --json`.
- Do not make saved repair JSON authorize writes without current validation or
  explicit override.
- Do not promote any remaining proposal axis into the specification before
  implementation and tests support it.

## Read When

- Changing the boundary between advisory candidate JSON and an applying
  command.
- Designing broader verification orchestration, ranking evidence, edit
  granularity, or application authority.
- Auditing whether new repair-loop behavior belongs in `check --json`,
  `repair`, `explain`, or a future command.
