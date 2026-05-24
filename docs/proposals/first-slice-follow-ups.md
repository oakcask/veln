# First-Slice Follow-Up Targets

Status: accepted-proposal
Implementation: partially implemented

This document tracks accepted first-slice targets that are not fully
implemented in the current workspace. The completed implementation sequence
stays in
[../phases/first-slice-implementation.md](../phases/first-slice-implementation.md).

## Language And Type Coverage

- `match` expressions are not implemented yet. Expected-type flow through
  match branches and match lowering remain follow-up work.
- Broader runtime semantics still need match lowering, stable callable value
  construction, and numeric behavior beyond matching `Int` or matching `Float`
  operator operands.

## Repair Loop

- `hole.unfilled` emits candidate-query records when an expected type is
  known, but candidate ranking and concrete repair generation remain outside
  the completed first-slice gate.
- `satisfy` suffix parsing, formatting, constraint exposure, missing candidate
  diagnostics, missing `=>` diagnostics, candidate shadowing diagnostics, and
  unused candidate diagnostics are implemented. Candidate ranking and concrete
  repair generation remain follow-up work before formatter stabilization.

## Effects And Contracts

- Direct stdio calls are recognized as compiler-known effectful prelude calls,
  private helper body effects propagate to callers, and effect diagnostics
  expose bounded path entries with hidden-frame and omitted-path counts.
- The checker validates the first-slice pure boolean contract subset, but
  runtime contract discharge remains deferred.
- Contract predicates now parse through a dedicated first-slice predicate
  production. Pure calls to discovered effect-free functions are validated;
  richer predicate semantics remain follow-up work.

## Formatting

- Comment-bearing files are currently preserved byte-for-byte. Comment
  attachment is still required before those files can be safely reformatted.
- Formatter stabilization still needs focused golden and idempotence fixtures
  for `ensure`, prefix and binary precedence, postfix `?`, nested
  records/lists/calls, multiple input files without parse errors, and
  comment attachment once comments stop being no-op preserved.

## Lowering And Execution

- Broader lowering stabilization still needs focused fixtures for
  function-typed value calls, blocked call and constructor arity cases, missing
  expression blockers, and selected-entry reachable-hole handling.
- Reachable-hole blocking currently follows the selected entry and direct
  function-name calls in expressions. Broader conservative handling for future
  higher-order values, module initializers, imports, and ambiguous graph edges
  remains follow-up work.
- A persistent build cache remains deferred.

## Test Discovery And Events

- Test discovery selects explicit top-level `test` declarations. Parsed
  docblock/example extraction, expected-output examples, and automatic
  same-file example discovery remain follow-up work.
- Test stabilization should add focused fixtures for static-gate parse and
  semantic diagnostics in `veln test --json` and multiple test files.
