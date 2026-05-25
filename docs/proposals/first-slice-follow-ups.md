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
  and direct inclusive comparison satisfy-constrained symbol repair candidates
  are generated as unapplied safe repair candidates when the predicate becomes
  reflexive for the same visible binding. Tautological equality and inclusive
  comparison predicates on the satisfy candidate itself mark every
  type-compatible visible binding candidate as an unapplied safe repair
  candidate, including parenthesized direct and tautological clauses. Satisfy
  predicates whose candidate substitution is already guaranteed by a valid
  `require` clause mark the matching visible binding as an unapplied safe
  repair candidate, including string-literal clauses and simple direct,
  commuted, parenthesized comparison clauses, and whole parenthesized
  `and` conjunctions. Broader repair discharge remains follow-up work before
  formatter stabilization.

## Effects And Contracts

- Direct stdio calls are recognized as compiler-known effectful prelude calls,
  private helper body effects propagate to callers, and effect diagnostics
  expose bounded path entries with hidden-frame and omitted-path counts.
- The executable bounded-channel slice is implemented and specified in the
  language reference. `spawn`, task handles, cancellation, join, selection,
  and concurrent stdio/test event ordering remain future concurrency surface
  work.
- The checker validates the first-slice pure boolean contract subset. Runtime
  contract discharge is implemented for function-entry `require` checks and
  `ensure` checks before both ordinary returns and `?` early returns.
  `veln test --json` reports runtime contract failures inside selected test
  cases as structured failed-case details, and `veln run --json` reports
  runtime contract failures as top-level structured errors.
- Contract predicates now parse through a dedicated first-slice predicate
  production. Bare and `use`-alias qualified pure calls to discovered
  effect-free functions are validated and participate in selected-entry
  reachability for executable commands. Richer predicate semantics remain
  follow-up work.

## Formatting

No accepted formatting follow-up is currently tracked here.

## Lowering And Execution

- Reachable-hole blocking follows the selected entry, direct function-name
  calls, bare function declaration values used in reachable expressions, and
  function calls in contract predicates. Qualified calls through `use` aliases
  now resolve reachability to functions in the imported source module without
  including same-named functions from other modules. Broader conservative
  handling for future higher-order values and module initializers remains
  follow-up work.

## Test Discovery And Events

No accepted test discovery follow-up is currently tracked here.
