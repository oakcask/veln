# First-Slice Follow-Up Targets

Status: accepted-proposal
Implementation: partially implemented

This document tracks accepted first-slice targets that are not fully
implemented in the current workspace. The completed implementation sequence
stays in
[../phases/first-slice-implementation.md](../phases/first-slice-implementation.md).

## Language And Type Coverage

No accepted language and type coverage follow-up is currently tracked here.

## Repair Loop

- `hole.unfilled` emits candidate-query records when an expected type is
  known and ranks visible assignable symbol candidates when available.
- `satisfy` suffix parsing, formatting, constraint exposure, missing candidate
  diagnostics, missing `=>` diagnostics, candidate shadowing diagnostics, and
  unused candidate diagnostics are implemented. Satisfy predicates are
  semantically validated against the pure boolean predicate subset with the
  candidate bound to the hole expected type when known. Direct equality
  satisfy-constrained symbol repair candidates are generated as unapplied safe
  repair candidates. Richer repair discharge remains follow-up work before
  formatter stabilization.

## Effects And Contracts

- Direct stdio calls are recognized as compiler-known effectful prelude calls,
  private helper body effects propagate to callers, and effect diagnostics
  expose bounded path entries with hidden-frame and omitted-path counts.
- The checker validates the first-slice pure boolean contract subset. Runtime
  contract discharge is implemented for function-entry `require` checks and
  ordinary-return `ensure` checks. `veln test --json` reports runtime contract
  failures inside selected test cases as structured failed-case details, and
  `veln run --json` reports runtime contract failures as top-level structured
  errors. Non-local return refinements remain follow-up work.
- Contract predicates now parse through a dedicated first-slice predicate
  production. Pure calls to discovered effect-free functions are validated;
  richer predicate semantics remain follow-up work.

## Formatting

No accepted formatting follow-up is currently tracked here.

## Lowering And Execution

- Reachable-hole blocking follows the selected entry, direct function-name
  calls, and bare function declaration values used in reachable expressions.
  Broader conservative handling for future higher-order values, module
  initializers, imports, and ambiguous graph edges remains follow-up work.
- A persistent build cache remains deferred.

## Test Discovery And Events

- Test discovery selects top-level `test` declarations from `*_test.veln`
  files, explicit targets, and same-file declarations in other discovered
  source files. Documentation comment `veln` doctest extraction and adjacent
  `veln-output` expected-output comparison are implemented. Explicit doctest
  `error=<TypePath>` metadata is implemented for generated result-returning
  wrappers. Unknown and invalid metadata diagnostics are implemented for
  executable doctest fences and expected-output fences. Doctests that contain
  `?` infer the wrapper error type from the immediately documented public
  function's explicit `Result(_, E)` return type when that context is
  available or from known propagated function calls when every propagated call
  has the same concrete error type. Non-runnable doctest examples are
  implemented with `veln ignore`. Hidden setup is implemented with `# ` lines
  inside executable doctest fences. Negative static examples are implemented
  with `veln fail`.
