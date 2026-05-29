# Proposal Target Selection

Status: routing

Use this page when a session asks for the current proposal target and no
concrete proposal page is named. This is a routing page, not an implementation
proposal.

## Read First

- Current implemented behavior:
  [../specification/README.md](../specification/README.md).
- Proposal area index: [README.md](README.md).
- Implementation mechanics after a target is selected:
  [implementation-route.md](implementation-route.md).

## Current Prompt State

No concrete proposal target is selected when no concrete target prompt is
present or the prompt state says no target is selected. That state has no
proposal completion conditions to implement.

Before changing code, choose an existing short proposal page or split one
narrow target out of the follow-up inventory. Return to [README.md](README.md)
after selecting the target area; use
[implementation-route.md](implementation-route.md) only after a concrete page
owns the target.

## Decision Flow

1. If a prompt names one concrete proposal page, read that page and compare it
   with the matching `../specification/` page before implementation.
2. If no concrete prompt is selected, stay on the candidate gates below.
3. If a candidate gate is still broad, split the smallest observable behavior
   into a short proposal page before implementation.
4. Use [implementation-route.md](implementation-route.md) only after the short
   proposal page states the selected target.

## Candidate Gates

This section is the single short list of no-target candidate gates. Proposal
indexes link here instead of copying these gate descriptions.

- Declarative harness work:
  [toolchain-test-harness-extensions.md](toolchain-test-harness-extensions.md)
  needs one manifest feature that replaces repeated bespoke CLI setup or
  assertion code across at least two command paths.
- Runtime-failure doctest work:
  [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md)
  needs one concrete runtime failure class with structured test JSON details
  and CLI coverage.
- Runtime path representation:
  [path-runtime-representation.md](path-runtime-representation.md)
  needs one observable `Path` behavior that host-string storage cannot
  express.
- Self-hosting standard-library work:
  [self-hosting-standard-library.md](self-hosting-standard-library.md)
  needs one descriptor-only pure helper selected from the implemented helper
  split.
- Repair-loop work:
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md)
  needs a new short proposal page for behavior beyond the implemented
  confirmation, override, multi-edit, and post-edit check boundary.

## Selection Checklist

1. Confirm the matching `../specification/` page does not already state the
   behavior.
2. Name one observable behavior, harness capability, helper migration, or
   repair workflow.
3. Keep the target small enough that tests, implementation, and specification
   promotion can move together.
4. Add or update the short proposal page before using implementation mechanics.
5. Keep `../specification/` unchanged until code and tests support the selected
   behavior.

## Update When

- A concrete target is selected for implementation.
- A candidate gate is satisfied, split, completed, superseded, or rejected.
- A proposal page changes status in a way that affects whether it can be the
  next target.

## Skip Unless Needed

- Do not copy candidate gates into proposal indexes; link here instead.
- Do not use [reference-followups.md](reference-followups.md) as an
  implementation target until a short proposal page names the concrete slice.
