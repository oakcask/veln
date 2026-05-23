---
name: github-pr-operations
description: Use when creating, editing, or managing pull requests with GitHub CLI, including stacked pull requests, PR metadata updates, labels, and retry behavior for GitHub API connection failures.
---

# GitHub PR Operations

## Goal

Use GitHub CLI in a way that preserves repository approval rules and avoids accidental shell wrapping.

## Workflow

1. Inspect the branch, diff, and existing PR state before creating or editing PR metadata.
2. Use `conventional-commits` for PR titles authored by Codex.
3. Use `change-rationale-writing` for PR descriptions authored by Codex.
4. Run `gh pr create` or `gh pr edit` directly.
5. If a direct `gh pr create` or `gh pr edit` fails with a GitHub API connection error, treat it as a sandbox network failure and retry the same direct command with the existing GitHub CLI approval prefix.

## Command Rules

- Do not wrap `gh pr create` or `gh pr edit` in `bash -lc`, shell functions, command substitutions, or inline environment assignments unless the user explicitly requests that form.
- Prefer direct GitHub CLI commands so command-prefix approval rules can match them.
- For Markdown PR descriptions, especially multiline bodies or text containing backticks, write the description to a body file and pass it with `--body-file`; do not pass the Markdown through shell-interpreted inline strings.
- When requesting escalation for `gh pr create` or `gh pr edit`, suggest only the stable prefix rule `["gh", "pr", "create"]` or `["gh", "pr", "edit"]`. Do not include repository, branch, title, body, label, or draft arguments in the suggested prefix rule.
- Avoid GitHub mention syntax in PR descriptions unless deliberately notifying a user or team.
- In PR descriptions, every command line must be wrapped in inline backticks or a fenced code block so command text containing `@` cannot create accidental mentions.
- In PR descriptions, prefer root scripts such as `pnpm build`, package directory names such as `packages/web`, or escaped scoped package names.

## Output

When reporting the result, include:

- PR URL or PR number when available.
- Title and important labels that were set.
- Verification commands or checks included in the PR description.
- Any API connection retry or approval issue that affected the operation.
