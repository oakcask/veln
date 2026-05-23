---
name: change-rationale-writing
description: Use when drafting or revising commit message bodies, pull request descriptions, merge request descriptions, changelog-style summaries, release note text, or decision records inside change descriptions. Focus the writing on the purpose, intent, consequences, tradeoffs, risks, and reviewer-relevant context behind code changes and durable project decisions rather than restating what the diff already shows.
---

# Change Rationale Writing

## Goal

When writing commit message bodies or pull request descriptions, explain why the change exists and what it means. For important choices, leave enough decision context that future reviewers can understand the accepted tradeoff without reconstructing the discussion. Do not spend most of the text restating implementation details that reviewers can read directly from the diff.

## Core Emphasis

Prioritize:

- **Intent**: What problem, user need, operational pain, or design goal motivated the change?
- **Reasoning**: Why is this approach appropriate compared with the likely alternatives?
- **Decision Record**: What durable choice is being made, what alternatives were rejected, and what would justify revisiting it?
- **Consequences**: What behavior, compatibility, workflow, performance, security, or maintenance effects should readers expect?
- **Risks**: What could still fail, regress, or surprise users and maintainers?
- **Confidence**: What evidence reduces risk, such as tests, manual checks, logs, rollout constraints, or prior incidents?

De-emphasize:

- File-by-file summaries.
- Function-by-function narration.
- Repeating renamed symbols, moved code, or mechanical edits unless those details explain intent or risk.
- Generic phrases such as "updated code", "made changes", or "refactored logic" without rationale.

## Workflow

Before drafting:

1. Inspect the actual diff, issue context, tests, and user request when available.
2. Infer the central intention of the change.
3. Identify durable decisions in the change, especially configuration, linting, dependency, API, data model, security, compatibility, or workflow choices.
4. Identify direct consequences for users, operators, developers, APIs, data, or deployment.
5. Identify residual risks, review expectations, and any verification performed.
6. Write the body or description so a future reader understands the decision without rereading the entire diff.

If the intent or risk cannot be inferred, state the uncertainty briefly instead of inventing a reason.

## Commit Message Bodies

Use a commit body when the subject alone does not explain the reason or implications. Keep it concise, usually one to three short paragraphs.

Good commit bodies answer:

- Why was this change necessary now?
- What important behavior or contract changes?
- What tradeoff or risk should future maintainers know?

Avoid bodies that only say what files changed.

## Pull Request Descriptions

Prefer this structure when no repository template exists:

```markdown
## Intent

...

## Consequences

...

## Risks

...

## Verification

...
```

Adapt headings to the repository's existing template. If a template asks for "Summary", use that section to describe intent and impact, not just implementation.

When a pull request changes public behavior or compatibility, mark the PR title with `!` and include a `BREAKING CHANGE: ...` line in the description. Put the line where the repository template discusses compatibility or consequences, and explain the removed, changed, or incompatible contract from the consumer's point of view.

## Decision Records in Change Descriptions

Use a pull request description as a lightweight decision record when the change makes a durable project choice, such as enabling or disabling a lint rule, adopting or rejecting a dependency, changing an API contract, choosing a compatibility policy, or accepting a known maintenance tradeoff.

A useful decision record answers:

- What decision is being made?
- Why is it appropriate for this repository now?
- What alternatives were considered or implied, and why are they not being used?
- What future behavior should reviewers enforce because of this decision?
- What risk remains, and what evidence or constraints make that risk acceptable?

Keep the record close to the affected PR section. A `Risks`, `Consequences`, `Compatibility`, or `Review Guidance` section is usually better than adding a separate process-heavy heading unless the repository already uses one.

Avoid:

- Treating every implementation detail as a decision.
- Hiding a major tradeoff in a vague summary bullet.
- Writing permanent-sounding policy when the evidence only supports a local or temporary choice.
- Omitting the review rule that makes the decision safe to maintain.

## Writing Style

- Write commit bodies, pull request descriptions, merge request descriptions, changelog entries, and release notes in English.
- If the source material or draft is in another language, translate the final published text into clear reviewer-facing English.
- Be specific and concrete.
- Tie risks to affected behavior, data, compatibility, or operations.
- Mention implementation details only when they clarify the rationale, consequence, or review focus.
- Keep claims proportional to evidence.
- Do not include environment-specific or personal information.
- Avoid GitHub mention syntax unless the purpose is to notify that user or team. For verification commands in PR descriptions, prefer root scripts like `pnpm build`, package paths like `packages/web`, or escaped scoped package names so package scopes do not become mentions.
- In PR descriptions, write every command line inside inline backticks or a fenced code block. This is required for verification entries too, because raw command text can contain `@` and accidentally notify users or teams.

## Examples

Weak:

```text
Updated the parser and added tests.
```

Better:

```text
The parser now treats empty input as a valid no-op so callers can pass optional configuration without pre-filtering. This keeps the API tolerant while preserving existing error behavior for malformed input.

The main risk is that downstream code may have relied on the previous exception path for empty input, so the tests cover both empty and invalid cases.
```

Weak:

```markdown
## Summary

- Changed auth middleware
- Updated session tests
```

Better:

```markdown
## Intent

Expired sessions were being reported as generic authorization failures, which made support and retry behavior harder to distinguish from genuine permission problems.

## Consequences

Clients now receive a session-specific failure path and can prompt reauthentication without treating the account as unauthorized.

## Risks

This changes an error classification that some callers may have matched directly. The compatibility risk is limited to expired-session handling.

## Verification

Covered the expired-session branch and the existing unauthorized branch in tests.
```

Decision record example:

```markdown
## Risks

Non-null assertions remain allowed deliberately. The current uses are narrow: checked non-empty array access and test fixtures that assert generated diagnostics have expected entries. Turning the rule on now would mostly force extra guards or helper wrappers around invariants that TypeScript cannot infer, without changing runtime behavior.

This keeps strict TypeScript as the primary null-safety gate while treating `!` as an explicit invariant assertion during review, not as a general escape hatch. The residual risk is future overuse, so reviewers should reject new assertions on external input, optional API results, DOM lookups, or async state unless the value is guarded first.
```
