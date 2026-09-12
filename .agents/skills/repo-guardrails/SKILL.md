---
name: repo-guardrails
description: Add, revise, or evaluate repository and agent guardrails, including their placement and enforcement.
---

# Repo Guardrails

## Goal

Use the smallest durable guardrail that meaningfully reduces the demonstrated
risk without constraining unrelated work.

## Assessment

Establish:

- the concrete failure mode and why it is durable rather than transitional
- the authority for the expected outcome
- whether the guard is advisory, procedural, enforceable, or combined
- the purpose, audience, and scope of existing repository surfaces
- the expected harm reduction, false positives, duplication, and maintenance
  cost

Do not count prose, a validator, and tests that copy the same expectation as
independent evidence. Prefer deleting or simplifying a confusing rule when that
removes the risk.

## Placement

When choosing between `AGENTS.md`, a skill, CI, tests, linters, templates, or
documentation, read [placement.md](references/placement.md). Inspect the
candidate surfaces before changing them and choose the narrowest effective
owner that matches existing repository boundaries.

Give a transitional guard a concrete removal condition. Do not make a one-time
migration condition permanent CI policy.

## Implementation

- Keep policy concise and semantic; do not encode exact prose layout, internal
  ordering, or one change's file list unless it is a consumed interface.
- Do not duplicate the same long rule across surfaces.
- Reuse existing checks and naming conventions before adding dependencies or
  automation.
- When enforcement is appropriate, verify that a representative violation
  actually fails and document the supported local check.
- Keep environment-specific and personal information out of repository
  artifacts.

## Report

State the risk, authority, chosen location, important rejected alternatives,
maintenance cost, retirement condition when transitional, and verification.
