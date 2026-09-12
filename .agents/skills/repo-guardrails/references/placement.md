# Guardrail Placement

## AGENTS.md

Use `AGENTS.md` for concise principles that should shape default behavior across
many tasks, especially judgment-based rules about communication, privacy,
editing, review posture, escalation, or deliverables.

Keep task-specific procedures, command recipes, checklists, troubleshooting,
and long rationale out of `AGENTS.md`. Link a skill only when agents must
discover the specialized workflow by default.

## Skills

Use a skill for a specific class of work that needs a reusable workflow,
decision tree, checklist, or domain context. Keep its selection description
short and its root document focused; route conditional detail to supporting
references.

Use `AGENTS.md` plus a skill when a broad principle also needs a specialized
workflow. This suits judgment-heavy work such as dependency or security review.

## CI, Tests, and Linters

Use CI when a durable invariant can be checked deterministically and should
apply to humans, agents, and automation. The expected result must come from an
authoritative artifact or observable behavior, not constants owned only by the
check.

Use existing tests or linters when they can express runtime behavior, generated
output, formatting, imports, dependency policy, or API contracts with lower
maintenance cost than custom CI.

Combine a skill with CI when agents need procedural guidance and merges need
deterministic enforcement. Combine `AGENTS.md` with CI when the rule must be
remembered during work as well as enforced.

## Templates and Documentation

Use templates or documentation for human workflow, review input, and ownership
guidance that does not control agent behavior. Confirm the target documentation
area owns that subject before adding policy there.

Prefer a skill when the content primarily teaches agents how to perform or
review repeatable work. Stability or importance alone does not make operational
guidance product documentation.

## Cost and Scope Checks

- Avoid permanent checks for hypothetical bypasses or one-time transitions.
- Cover semantic failure classes rather than enumerating every mutation.
- Do not use paragraph positions, headings, or step names as completeness
  proxies unless another tool consumes that structure.
- Prefer a focused local rule over broad new automation.
- Give transitional guards an owner and observable retirement trigger when they
  cannot be removed immediately.
