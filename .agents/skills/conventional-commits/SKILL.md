---
name: conventional-commits
description: Require Conventional Commits for git commit messages and PR titles authored by Codex. Use whenever Codex is asked to create, amend, squash, rewrite, or otherwise author a commit message or PR title, including direct requests like "commit this", "make a commit", "git commit", "amend the commit", "open a PR", or "create a PR".
---

# Conventional Commits

## Overview

Use Conventional Commits for every commit message and PR title authored by Codex. Choose a clear type, optional scope, and concise imperative summary based on the actual staged changes or PR contents.

## Message And Title Rules

Use this format:

```text
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

- Use common types such as `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `chore`, or `revert`.
- Use `chore(ci)` for CI-related changes, including workflow, pipeline, report parsing,
  test sharding, budget checks, and automation updates. Do not use `feat` or
  `fix` with CI-related scopes such as `ci`, `workflow`, `workflows`, or `actions`
  for CI-only changes.
- Reserve `feat` and `fix` for product, runtime, API, or user-visible changes.
  Use `feat` only when adding product capability, and use `fix` only when
  correcting product behavior.
- Use `chore` for agent-instruction changes, including `AGENTS.md` and skills updates.
- When the type is `docs`, do not include a scope in commit subjects or PR
  titles. Use `docs: clarify setup`, not `docs(readme): clarify setup`, even
  when the changed document has a clear area.
- Add a scope only when it names the affected component, package, subsystem, surface, or document, for example `fix(auth): handle expired tokens`.
- Prefer scopes such as `cli`, `core`, `api`, `parser`, `readme`, or `agents` when they match the actual area changed.
- Do not use project phases, milestones, ticket names, branch names, release labels, or vague workstream labels as scopes. Examples to avoid: `phase`, `phase-1`, `milestone`, `sprint`, `ticket-123`, `complete-phase-1`.
- If no concrete component or surface name fits cleanly, omit the scope instead of inventing a thematic label.
- Use `!` and a `BREAKING CHANGE:` footer when the change breaks public behavior or compatibility.
- Keep the subject short, lower-case after the type when natural, and do not end it with a period.
- Use an imperative description, for example `fix(parser): handle empty input`.
- Write the full commit message and PR title in English, even when the user request, notes, or intermediate draft are in another language.

## Workflow

Before creating or amending a commit:

1. Inspect the staged diff and repository status.
2. Infer the primary intent of the staged changes.
3. Select the most specific conventional type and, if useful, a scope that names the changed component or surface.
   If the staged changes only affect `.github/workflows`, `workflow-scripts`, CI
   policy, or CI automation tests, use `chore(ci)`, not `feat` or `fix`.
   If the selected type is `docs`, omit the scope.
4. If multiple unrelated intents are staged, tell the user and either split commits when asked or choose the dominant intent.
5. Create the commit with a Conventional Commit message.

Before creating or updating a PR title:

1. Inspect the PR diff or branch changes, but do not derive the scope from the branch name when the changed area is more specific.
2. Infer the primary intent of the PR.
3. Select the most specific conventional type and, if useful, a scope that names the changed component or surface.
   If the PR only changes `.github/workflows`, `workflow-scripts`, CI policy, or CI
   automation tests, use `chore(ci)`, not `feat` or `fix`.
   If the selected type is `docs`, omit the scope.
4. Write the PR title as a single Conventional Commit subject line.
5. If the PR combines unrelated intents, tell the user and choose the dominant intent unless asked to split the work.

## Examples

- `feat(cli): add dry-run option`
- `fix(api): preserve request headers`
- `docs: clarify setup steps`
- `refactor(cache)!: replace storage format`
- `feat: complete phase 1 validation`
- `docs: document phase 1 diagnostics`
