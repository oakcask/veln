# Advisory Repair Candidates

This page is the entry point for implemented advisory repair candidates. Start
here when a task mentions repair candidates, safe repair, candidate edits, or a
future repair command.

Candidate records may appear in `veln check --json`; they are not an applying
repair workflow and are not a specification for a dedicated repair command.

## Read First

- No `veln repair` command is implemented. Candidate edits are concrete
  replacement suggestions inside diagnostics, but command execution leaves them
  unapplied.
- Candidate application policy is evidence and review routing. Even
  `safe_repair_candidate` means the implemented static subset has discharged;
  it does not authorize automatic edit application.

## Detail Routes

- Exact candidate ranking, `satisfy` repair constraints, and safe-repair
  matching rules: [holes.md](holes.md).
- Stable `check --json` envelope, diagnostic fields, and candidate `details`
  payload boundaries: [diagnostics-json.md](diagnostics-json.md).
- Implemented command availability and the explicit absence of `veln repair`:
  [commands.md](commands.md).
- Rationale for keeping candidates advisory until an edit is authorized:
  [source-decisions.md](source-decisions.md).
- Proposed command invocation, confirmation, override, and application
  behavior:
  [../proposals/agent-language-spec-wall/repair-command.md](../proposals/agent-language-spec-wall/repair-command.md).

## Read When

- Changing candidate record fields, ranking, edits, evidence, known limits,
  blocking obligations, verification hints, or application policy.
- Deciding whether repair-loop behavior belongs in implemented `check --json`
  diagnostics or remains proposal work for a future command.
- Keeping the repair command proposal subordinate to current implemented
  behavior.

## Skip Unless Needed

- Use [holes-full.md](holes-full.md) only for exact candidate examples or
  matching rules.
- Use [diagnostics-json-full.md](diagnostics-json-full.md) only for the full
  diagnostic field catalog.
- Do not promote command invocation, confirmation, override, or automatic
  application behavior into this specification until it is implemented.
- Open
  [../reference/source-decisions/records/result-safe-repair-candidate-boundary.md](../reference/source-decisions/records/result-safe-repair-candidate-boundary.md)
  only when the advisory-versus-application rationale is needed.
