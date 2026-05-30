# Doctest Runtime Failure Expectations

Status: proposed follow-ups

This page records remaining runtime-failure follow-up work outside the
implemented contract-failure doctest route. Current behavior is specified in
`../specification/`.

## Read First

- Current doctest command behavior:
  [../specification/commands.md](../specification/commands.md).
- Current doctest JSON case shape:
  [../specification/test-json.md](../specification/test-json.md).
- Current runtime contract failure shape:
  [../specification/contracts.md](../specification/contracts.md).
- Proposal mechanics and promotion checks:
  [target-selection.md](target-selection.md) and
  [implementation-route.md](implementation-route.md).

## Current Boundary

`veln fail` doctest fences are static negative examples. They are accepted only
when generated source produces an error diagnostic before execution. They do
not describe expected runtime failures.

Positive doctests may use `runtime=contract` metadata to expect a runtime
contract failure. Broader panic matching, arbitrary stderr matching, and
command-status assertions remain outside the implemented surface.

## Target

Expand runtime failure expectations beyond the initial contract-failure route
only when a concrete failure class has structured test JSON details and CLI
coverage.

## Work Route

- Start from the implemented `test` and doctest rules in
  [../specification/commands.md](../specification/commands.md) and
  [../specification/test-json.md](../specification/test-json.md).
- Reuse existing structured runtime failure records where possible rather than
  inventing a second failure shape.
- Keep expected-output comparison independent from expected runtime failure
  matching; one route decides output text, the other decides runtime failure
  kind and details.
- Promote additional behavior into `../specification/` only after code and CLI
  coverage prove the new metadata, pass condition, mismatch condition, and
  blocked condition.

## Non-Goals

- Do not change `veln fail`; it remains a static expected-error facility.
- Do not add broad panic matching, arbitrary stderr matching, or command-status
  assertions in the first target.
- Do not weaken parse, semantic, or checked-core gates before runtime
  execution.

## Acceptance Checks

- A positive static doctest with the additional runtime-failure metadata is
  selected and executed.
- The doctest passes only when the selected runtime failure kind and key details
  match.
- If execution succeeds, fails with a different runtime failure, or is blocked
  before execution, the doctest case fails or blocks with structured test JSON.
- Expected-output comparison remains separate from expected runtime failure
  matching.

## Update When

- Add any newly implemented metadata and test JSON behavior to
  `../specification/commands.md` and `../specification/test-json.md` only after
  code and CLI coverage exist.
