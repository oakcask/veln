---
name: repo-guardrails
description: Use when the user wants to add, revise, or evaluate guardrails for agent behavior, repository policies, AGENTS.md instructions, skills, CI checks, generated outputs, workflow safety, or enforcement mechanisms. Helps choose whether the guardrail belongs in AGENTS.md, a skill, CI, tests, linters, templates, documentation, or a combination.
---

# Repo Guardrails

## Goal

Add the smallest guardrail that meaningfully reduces the risk the user cares about. Prefer repository-local conventions and existing tooling over introducing a new policy surface.

## Workflow

1. Identify the risk, failure mode, or behavior the user wants to prevent.
2. Classify the guardrail as advisory, procedural, enforceable, or a combination.
3. Inspect the relevant repository surfaces before changing them, such as
   `AGENTS.md`, existing skills, CI workflows, test configuration, lint
   configuration, templates, and the README or index for any documentation area
   under consideration.
4. Identify each candidate location's existing purpose, audience, and scope
   boundary.
5. Choose the narrowest effective location that fits those local boundaries.
6. Implement the guardrail in the chosen location with concise wording or focused automation.
7. Verify that the guardrail is discoverable and, when enforceable, that the check can actually fail on violations.

If the user is only brainstorming, discuss the placement and tradeoffs without editing files.

## Placement Guide

Default to progressive disclosure: keep `AGENTS.md` as a short entry point for always-on principles and skill discovery. Do not add task-specific procedures, command recipes, checklists, or troubleshooting flows to `AGENTS.md`; create or update a skill instead, then link to it from `AGENTS.md` only when agents must discover it by default.

Use `AGENTS.md` when:

- The rule should shape default agent behavior across many tasks.
- The rule is judgment-based or difficult to check mechanically.
- The rule affects communication style, privacy, file editing behavior, review posture, escalation, or deliverables.

Use a skill when:

- The rule applies only to a specific class of work.
- The agent needs a repeatable task workflow, decision tree, checklist, or domain-specific context.
- Adding the rule to `AGENTS.md` would create noise for unrelated tasks.

Use CI when:

- The rule can be checked deterministically.
- Violations should block merges or releases.
- The same constraint should apply to humans, agents, and automation.

Use tests or linters when:

- The invariant belongs to runtime behavior, generated output, source formatting, imports, dependency policy, or API contracts.
- Existing test or lint infrastructure can express the rule with lower maintenance cost than a custom CI step.

Use templates or documentation when:

- The guardrail guides human workflow but does not need to control agent behavior directly.
- The risk is mostly missing context, inconsistent review input, or unclear ownership.
- The target documentation area already owns that kind of policy. Do not put a
  guardrail in a docs area just because it is stable; first confirm the local
  README or index says that area covers the subject and audience.

Prefer a skill over documentation when:

- The guardrail primarily teaches agents how to do or review a repeatable task.
- The policy is operational workflow guidance rather than product, language,
  API, architecture, or other durable project reference material.
- The only reason to use documentation is that the rule feels stable or
  important.

## Combination Patterns

- Use `AGENTS.md` plus CI when the agent needs to remember the rule and the repository can enforce it.
- Use a skill plus CI when the agent needs a task-specific procedure and the result needs deterministic validation.
- Use `AGENTS.md` plus a skill when a broad principle has a specialized implementation workflow.
- Prefer `AGENTS.md` plus a skill for judgment-heavy rules that must shape default behavior but also need a repeatable deep-review workflow, such as third-party dependency selection and security review.
- Use documentation plus templates when the guardrail is primarily about human coordination.

## Implementation Rules

- Keep policy text short and concrete.
- Before expanding `AGENTS.md`, check whether the content is a task-specific procedure that belongs in a skill.
- When editing `AGENTS.md`, also audit the touched section for existing task-specific procedures, command recipes, checklists, troubleshooting flows, or long rationale. Move those details into the relevant skill, or create a focused skill when no suitable one exists, and replace the AGENTS entry with a short always-on rule or skill discovery link.
- Before adding a guardrail to documentation, read that docs area's README or
  routing page and confirm the new rule belongs to that area's stated scope. If
  the docs area is scoped to a product or language domain, do not add unrelated
  repository-operation policy there.
- Do not duplicate the same long rule across multiple files; put the principle in one place and enforcement in another when needed.
- Avoid environment-specific or personal information in examples, generated files, comments, tests, logs, and commit messages.
- Reuse existing CI jobs, scripts, lint tools, and repository naming conventions when practical.
- Do not add new dependencies or broad automation unless the risk justifies the maintenance cost.
- When adding an enforcement check, include at least one way to run or validate it locally if the repository supports that pattern.

## Response Shape

When proposing or implementing a guardrail, explain briefly:

- The risk being addressed.
- The chosen location and why.
- Any important alternatives rejected.
- How the guardrail was or should be verified.
