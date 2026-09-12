---
name: change-rationale-writing
description: Write commit bodies, pull request descriptions, release notes, or change decision records that explain rationale and consequences.
---

# Change Rationale Writing

## Goal

Explain why a change exists, what it means, and which tradeoffs future reviewers
must understand. Do not spend most of the text narrating a diff.

## Shared Guidance

Cover the questions that matter for the change:

- What behavior, contract, decision, or project state changes?
- Why is the change needed, and why is this approach appropriate?
- Why not keep the status quo or choose a likely alternative, when that choice
  would reasonably concern a reviewer?
- What consequences and residual risks affect users or maintainers?
- What evidence supports material claims?

Do not invent rationale, alternatives, confidence, or risks. State uncertainty
when the available request, diff, issue context, and verification do not
establish them.

## Deliverable Routes

- For a commit message body, read
  [commit-bodies.md](references/commit-bodies.md).
- For a pull request or merge request description, including a lightweight
  decision record, read
  [pull-request-descriptions.md](references/pull-request-descriptions.md).
- For a changelog entry or release note, apply the shared guidance directly.
  Lead with user-visible behavior and compatibility; omit internal mechanics
  unless they explain impact or migration work.

## Style

- Write published change descriptions in English.
- Be concrete and keep claims proportional to evidence.
- Mention implementation details only when they clarify rationale, impact, or
  review focus.
- Do not include environment-specific or personal information.
- Avoid GitHub mention syntax unless deliberately notifying its target.
