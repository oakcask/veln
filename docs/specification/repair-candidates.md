# Advisory Repair Candidates

This page is the entry point for the implemented boundary around advisory
repair candidates. Start here when a task mentions repair candidates, safe
repair, candidate edits, applying edits, or a future repair command.

## Current Boundary

- Candidate records may appear in `veln check --json` diagnostics. They are
  advisory records, not an applying workflow.
- Candidate edits are concrete replacement suggestions for the reported span,
  but command execution leaves them unapplied.
- Candidate application policy is evidence and review routing. Even
  `safe_repair_candidate` means the implemented static subset has discharged;
  it does not authorize automatic edit application.
- No `veln repair` command or repair option is implemented. Invocation,
  confirmation, override, and edit application remain proposal work.

## Choose Detail

- Candidate fields, stable `check --json` envelope, diagnostic spans, and
  `details` payload boundaries: [diagnostics-json.md](diagnostics-json.md).
- Candidate ranking, `satisfy` repair constraints, safe-repair matching, and
  exact examples: [holes.md](holes.md).
- Implemented command availability and command gates: [commands.md](commands.md).
- Rationale for keeping candidates advisory until edit application is
  authorized:
  [source-decisions.md](source-decisions.md).
- Proposed command invocation, confirmation, override, and applying workflow:
  [../proposals/agent-language-spec-wall/repair-command.md](../proposals/agent-language-spec-wall/repair-command.md).

## Read When

- Changing candidate record fields, ranking, edits, evidence, known limits,
  blocking obligations, verification hints, or application policy.
- Deciding whether repair-loop behavior belongs in implemented `check --json`
  diagnostics or remains proposal work for a future command.
- Auditing that proposal text stays subordinate to current implemented
  behavior.

## Skip Unless Needed

- Use [holes-full.md](holes-full.md) only for exact candidate examples or
  matching rules.
- Use [diagnostics-json-full.md](diagnostics-json-full.md) only for the full
  diagnostic field catalog.
- Open
  [../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  only when the advisory-versus-application rationale is needed.

## Promotion Rule

Do not promote command invocation, confirmation, override, dry-run/apply modes,
multi-file edit application, or automatic repair behavior into this
specification until the behavior is implemented and tested. Until then, keep
that material in proposal text.
