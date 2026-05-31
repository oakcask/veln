# Doctest Runtime Failure Follow-Ups

Status: proposed follow-ups

This page records remaining runtime-failure expectation work outside the
implemented doctest routes. Current behavior is specified in
`../specification/`; this page is only for adding another structured runtime
failure kind after a concrete failure class is selected.

## Read First

- Current doctest command behavior:
  [../specification/commands.md](../specification/commands.md).
- Current doctest metadata syntax, including implemented runtime expectation
  kinds:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current doctest JSON case shape, including runtime expectation mismatch
  records:
  [../specification/test-json.md](../specification/test-json.md).
- Current runtime failure details:
  [../specification/contracts.md](../specification/contracts.md).
- Current readable CLI coverage:
  [../specification/test-json.md](../specification/test-json.md).
- Proposal promotion checks:
  [implementation-route.md](implementation-route.md).

## Read When

- Changing runtime expectation matching:
  `crates/veln-test/src/runtime_expectation.rs`.
- Changing doctest runtime metadata parsing or diagnostics:
  `crates/veln-test/src/lib.rs`.
- Adding readable CLI coverage:
  `examples/specification/test/`.

## Current Boundary

`veln fail` doctest fences are static negative examples. They are accepted only
when generated source produces an error diagnostic before execution. They do
not describe expected runtime failures.

Positive doctests may use implemented runtime expectation metadata described in
`../specification/`. Broader panic matching, arbitrary stderr matching, and
command-status assertions remain outside the implemented surface.

Runtime failure expectations and expected-output fences are separate doctest
expectation routes. Runtime matching decides only failure kind and details;
output comparison decides only captured stdout or stderr text.

## Follow-Up Target

No additional runtime failure expectation kind is selected here. Add one only
when that failure class has structured test JSON details, metadata diagnostics,
and readable CLI coverage.

## Work Route

- Start from the implemented `test` and doctest rules in
  [../specification/commands.md](../specification/commands.md) and
  [../specification/test-json.md](../specification/test-json.md).
- Use [../specification/source-surface.md](../specification/source-surface.md)
  for metadata syntax and static doctest metadata diagnostics.
- Reuse existing structured runtime failure records where possible rather than
  inventing a second failure shape.
- Keep expected-output comparison independent from expected runtime failure
  matching; one route decides output text, the other decides runtime failure
  kind and details.
- Keep runtime expectation parsing and diagnostics as one attribute model:
  the same runtime kind should define its allowed metadata, required metadata,
  expected JSON shape, pass condition, mismatch condition, and blocked
  condition.
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

- A positive executable doctest with the additional runtime-failure metadata is
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
