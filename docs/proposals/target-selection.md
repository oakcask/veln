# Proposal Target Selection

Status: routing

Use this page when a session asks for the current proposal target and no
concrete target file is named. It is a routing page, not an implementation
proposal. Return to [README.md](README.md) after choosing or creating one
short proposal target.

## Read First

- Current implemented behavior: [../specification/README.md](../specification/README.md).
- Proposal implementation mechanics:
  [implementation-route.md](implementation-route.md).
- Mixed follow-up inventory:
  [reference-followups.md](reference-followups.md).

## Current Target

No concrete proposal target is selected. Do not begin implementation from the
broad follow-up inventory alone; first choose an existing short proposal page
or split a narrow target out of that inventory.

## Candidate Gates

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
4. Add or update the short proposal page before using
   [implementation-route.md](implementation-route.md).

## Update When

- A concrete target is selected for implementation.
- A candidate gate is satisfied, split, completed, superseded, or rejected.
- A proposal page changes status in a way that affects whether it can be the
  next target.
