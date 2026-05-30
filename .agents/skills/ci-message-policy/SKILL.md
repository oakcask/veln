---
name: ci-message-policy
description: Use when Codex adds, changes, reviews, or explains CI-visible messages, including GitHub Actions step names, failure summaries, annotations, report comments, log lines intended to guide maintainers or agents, CI policy text, and workflow output that asks someone to act.
---

# CI Message Policy

## Goal

Make CI-visible messages actionable without making maintainers reconstruct the
policy behind the check. Each message should answer what to do next and why the
action protects the project, either directly or through nearby evidence.

## Workflow

1. Identify the audience: maintainer, reviewer, release operator, or agent.
2. Identify the failed fact, threshold, or policy decision the message reports.
3. Write the required next action when a reasonable action exists.
4. Add why the action matters when the consequence is not obvious from the
   failed fact.
5. Point to the smallest useful evidence: command output, artifact name, report
   section, check name, or linked policy.
6. Keep implementation details, provenance, and longer explanation out of the
   headline. Put them in details, logs, artifacts, related output, or the PR
   description.
7. Verify the rendered output when the change affects generated comments,
   annotations, multiline logs, or truncation-sensitive text.

## Message Shape

Prefer this order for user-facing CI output:

1. Failed fact or required action.
2. Evidence needed to act.
3. Why the action matters, when needed.

Use concrete verbs such as inspect, rerun, update, split, remove, approve, or
revert. Avoid messages that only describe CI internals, such as "script failed"
or "threshold exceeded", unless no better repair action is known.

## Review Rule

When reviewing CI message changes, reject text that leaves either question
unanswered:

- What should the maintainer or agent do next?
- Why does that action protect the project?

If only one answer belongs in the visible message, the other must be clear from
nearby evidence such as the job name, artifact, report section, or linked
policy.

## Examples

Prefer:

- "Inspect `slow-test-files` before rerunning; one long file can hide shard
  health."
- "Update the diagnostic fixture after confirming the source behavior is
  intentional."
- "Split this test file before raising the timeout."

Avoid:

- "Script failed."
- "Threshold exceeded."
- "See logs."
