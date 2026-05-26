# Contracts

This page routes contract-specific language behavior. Open the full detail only
for exact rules or examples.

## Read First

- [Predicate syntax and validation](contracts-full.md#predicate-syntax-and-validation)
  defines implemented `require`, `ensure`, and `invariant` clauses.
- [Runtime obligations](contracts-full.md#runtime-obligations) defines
  contract enforcement and blame.
- [Static obligation classification](contracts-full.md#static-obligation-classification)
  defines static proof limits.
- [Explicit result bindings](contracts-full.md#result-binding) defines when
  `ensure` predicates may name the returned value.

## Read When

- Use this page before changing contract syntax, predicate validation, static
  contract gates, runtime contract checks, or contract diagnostics.
- Use [holes.md](holes.md) only when the contract change affects repair
  constraints for holes.

## Current Static Classification Note

Contract obligation classification is summarized here only for routing. Open
[Static obligation classification](contracts-full.md#static-obligation-classification)
when changing static proof rules, truth-table folding, case-split predicates,
ordering implications, equality or disequality consequents, order/equality
contradictions, disequality strict-order splits, numeric literal bound checks
including equality aliases, or literal-equality contradiction checks.

## Skip Unless Needed

- Skip [contracts-full.md](contracts-full.md) unless you need exact rules or
  examples.
- Use [source-surface.md](source-surface.md#contract-predicates) for the source
  grammar surface before reading older decision records.
