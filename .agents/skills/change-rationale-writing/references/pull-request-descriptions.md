# Pull Request Descriptions

Use the repository template when present. Otherwise prefer:

```markdown
## Intent

## Consequences

## Risks

## Compliance and Revisit Triggers
```

## Content

- **Intent:** State the problem or decision, why it matters, and why the status
  quo or a likely alternative is unsuitable.
- **Consequences:** Describe effects on users and maintainers and explain a
  material scope boundary.
- **Risks:** Name what could still fail or surprise readers and why the remaining
  risk is acceptable.
- **Compliance and Revisit Triggers:** Explain how future changes preserve each
  material decision and what observable change would invalidate its premise.

Treat the final section as decision-lifecycle guidance, not an execution
transcript. Name a regression scenario, assertion, fixture, specification case,
invariant check, or concrete review rule and explain which decision boundary it
protects. An unchanged test counts only when the description identifies its
relevant assertion.

Use review-only enforcement when automation would be disproportionate. State
the observable noncompliance condition and the residual risk. Do not invent a
trigger merely to fill the section or use a calendar reminder in place of an
observable condition.

Do not list commands or current outcomes that CI can report, including passing
tests, formatting, linting, compilation, or generated-file checks.

## Compatibility

When a pull request breaks public behavior or compatibility, use `!` in its
Conventional Commit title and include a `BREAKING CHANGE: ...` line where the
description discusses consequences. Describe the incompatible contract from
the consumer's point of view.

## Lightweight Decision Records

Use the description as a decision record when the change adopts a durable
project choice. Capture:

- the decision and why it fits the repository
- meaningful alternatives and why they were rejected
- consequences and remaining risk
- the rule future reviewers should enforce
- observable conditions that should cause reconsideration

Do not treat every implementation detail as a decision or make local evidence
sound like permanent policy.

## GitHub Safety

- Put every command line in inline code or a fenced block so `@` cannot create
  an accidental mention.
- Prefer root scripts or package paths over raw package identifiers when they
  communicate the same information.
- Avoid mention syntax unless deliberately notifying a user or team.
