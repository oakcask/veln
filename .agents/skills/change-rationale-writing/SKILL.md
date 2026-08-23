---
name: change-rationale-writing
description: Use when drafting or revising commit message bodies, pull request descriptions, merge request descriptions, changelog-style summaries, release note text, or decision records inside change descriptions. Focus the writing on the purpose, intent, consequences, tradeoffs, risks, and reviewer-relevant context behind code changes and durable project decisions rather than restating what the diff already shows.
---

# Change Rationale Writing

## Goal

When writing commit message bodies or pull request descriptions, explain why the change exists and what it means. For important choices, leave enough decision context that future reviewers can understand the accepted tradeoff without reconstructing the discussion. Do not spend most of the text restating implementation details that reviewers can read directly from the diff.

## What, Why, Why Not

Frame the change description around three questions:

- **What** changes, is decided, or remains true?
- **Why** is the change needed, and why is this approach appropriate?
- **Why not** keep the status quo, use a likely alternative, expand the scope,
  or add another mitigation or check?

Apply these questions across the whole description. Do not add the three labels
mechanically to every section or invent rejected alternatives to complete the
pattern. Include `Why not` when a reviewer could reasonably question an
alternative, omission, scope boundary, or accepted tradeoff.

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
2. Identify what behavior, contract, decision, or project state changes.
3. Explain why the change is needed and why the chosen approach fits.
4. Identify meaningful alternatives, omissions, and scope boundaries, and
   explain why they are not part of the change when that context aids review.
5. Identify direct consequences for users, operators, developers, APIs, data,
   or deployment.
6. For each material claim, identify the concrete change that supplies evidence,
   the observed result, and how that result supports the claim. Distinguish
   behavior-relevant verification from incidental repository hygiene checks.
   For pull request descriptions, apply the `Compliance and Revisit Triggers`
   rules below instead of recording results that CI can report.
7. Write the body or description so a future reader understands the decision without rereading the entire diff.

If the intent or risk cannot be inferred, state the uncertainty briefly instead of inventing a reason.

## Commit Message Bodies

Use a commit body when the subject alone does not explain the reason or implications. Keep it concise, usually one to three short paragraphs.

Good commit bodies answer:

- What important behavior, contract, or decision changes?
- Why was this change necessary, and why was this approach chosen?
- Why was a likely alternative or adjacent change left out, when relevant?
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

## Compliance and Revisit Triggers

...
```

Adapt headings to the repository's existing template. If a template asks for "Summary", use that section to describe intent and impact, not just implementation.

Use the framework across the sections:

- **Intent**: State what problem or decision the change addresses, why it should
  be addressed, and why the status quo or a likely alternative is unsuitable.
- **Consequences**: State what changes for affected users, operators, or
  maintainers, why those effects are acceptable, and why the scope is not
  broader or narrower when that boundary matters.
- **Risks**: State what could still fail or surprise maintainers, why the risk is
  acceptable, and why further mitigation is not included when reviewers might
  expect it.
- **Compliance and Revisit Triggers**: State how future changes remain compliant
  with each material decision or claim, and identify observable conditions that
  should cause maintainers to reconsider the decision.

Treat `Compliance and Revisit Triggers` as decision-lifecycle guidance, not as
an execution transcript or a copy of CI results.

For each material decision or claim, name the compliance mechanism at a useful
review granularity. Examples include a regression scenario and its assertion, a
fixture and expected output, a specification case, an invariant check, an
architecture fitness function, or a concrete review rule. Explain which future
violation the mechanism detects or prevents. An unchanged test can be a
compliance mechanism only when the description identifies the relevant scenario
or assertion and explains which decision boundary it protects.

Name concrete revisit triggers that would invalidate the decision's assumptions
or change its accepted tradeoff. Useful triggers include a changed public
contract, an upstream capability that removes the original constraint, a
measurable cost crossing an accepted bound, or evidence that the chosen behavior
no longer serves its users. A trigger starts a new decision; it does not require
automatic rollback. Call it an exit or rollback condition only when that
response is already part of the decision.

Do not turn this mapping into a file-by-file summary. Group related decisions
and compliance mechanisms by behavior, contract, or risk. If no durable
mechanism protects a material decision, state the reviewer-facing rule that
future changes must follow. If relying on review alone is intentional, explain
why automation would be disproportionate and state the remaining risk.

Do not invent a trigger merely to fill the section, use a calendar reminder as
a substitute for an observable condition, or write vague phrases such as
"revisit if needed." When no decision-specific trigger is known, state which
premise must materially change before reconsideration is warranted.

Do not include the outcome of a check that CI can run or report. Passing tests,
formatting, linting, compilation, generated-file checks, and similar current-run
results belong in CI, even when they are relevant to the change. Do not list the
commands for those checks either. Describe the enduring scenario, assertion, or
rule that establishes compliance instead of reporting that it passed.

Use a manual review rule only when compliance cannot reasonably be represented
by CI. Explain the observable condition and how future reviewers can recognize
noncompliance; do not reduce the entry to a one-time "verified" result. If
neither automation nor a repeatable review rule is practical, state that gap and
the remaining uncertainty.

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
- Avoid GitHub mention syntax unless the purpose is to notify that user or team.
  When a command is decision-relevant elsewhere in the description, prefer root
  scripts like `pnpm build`, package paths like `packages/web`, or escaped scoped
  package names so package scopes do not become mentions.
- In PR descriptions, write every command line inside inline backticks or a
  fenced code block, because raw command text can contain `@` and accidentally
  notify users or teams. The `Compliance and Revisit Triggers` section has the
  stricter no-command rule above.

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
Keeping the shared failure path would preserve that ambiguity; changing session
validation itself is unnecessary because only the resulting classification is
wrong.

## Consequences

Clients now receive a session-specific failure path and can prompt reauthentication without treating the account as unauthorized.
Permission failures remain unchanged because they require different client
guidance and are outside this classification fix.

## Risks

This changes an error classification that some callers may have matched directly. The compatibility risk is limited to expired-session handling.
No compatibility alias is retained because it would keep expired sessions
indistinguishable from genuine authorization failures.

## Compliance and Revisit Triggers

The expired-session regression is the compliance mechanism for the new
classification: it requires an expired session to produce the reauthentication
response. The retained permission-failure assertion protects the other side of
the boundary by requiring genuine permission failures to remain unauthorized.
Future changes that collapse either response into the shared path violate these
assertions.

Revisit the classification if the public protocol defines a standard
session-expiry response or supported clients can no longer distinguish the two
recovery paths. Either change invalidates the compatibility rationale for
maintaining separate responses.
```

Weak compliance guidance:

```markdown
## Compliance and Revisit Triggers

- `git diff --check`
- `cargo fmt --check`
- `cargo test`
```

Better compliance guidance:

```markdown
## Compliance and Revisit Triggers

The empty-input regression enforces the new parser contract by requiring a
successful no-op result. The adjacent malformed-input assertion requires
malformed input to remain an error. Together they make any future change that
conflates empty and malformed input a compliance failure.

Revisit this contract if callers can no longer treat empty input as an optional
configuration or if a later parser stage requires at least one token. Either
change removes the premise that a successful no-op is the safer boundary.
```

Decision record example:

```markdown
## Risks

Non-null assertions remain allowed deliberately. The current uses are narrow: checked non-empty array access and test fixtures that assert generated diagnostics have expected entries. Turning the rule on now would mostly force extra guards or helper wrappers around invariants that TypeScript cannot infer, without changing runtime behavior.

This keeps strict TypeScript as the primary null-safety gate while treating `!` as an explicit invariant assertion during review, not as a general escape hatch. The residual risk is future overuse, so reviewers should reject new assertions on external input, optional API results, DOM lookups, or async state unless the value is guarded first.
```
