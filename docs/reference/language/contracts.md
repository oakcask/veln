# Contracts

This page routes contract-specific language behavior. It points into the full
combined detail until the contract body is split further.

## Read First

- [Predicate syntax and validation](contracts-holes-full.md#predicate-syntax-and-validation)
  defines implemented `require`, `ensure`, and `invariant` clauses.
- [Runtime obligations](contracts-holes-full.md#runtime-obligations) defines
  contract enforcement and blame.
- [Static obligation classification](contracts-holes-full.md#static-obligation-classification)
  defines static proof limits.
- [Explicit result bindings](contracts-holes-full.md#result-binding) defines when
  `ensure` predicates may name the returned value.

## Read When

- Use this page before changing contract syntax, predicate validation, static
  contract gates, runtime contract checks, or contract diagnostics.
- Use [holes.md](holes.md) only when the contract change affects repair
  constraints for holes.

## Current Static Classification Note

Contract obligation classification statically proves top-level `or`
predicates where a negated disjunction is covered by repeated top-level
branches, such as `not (flag or ready) or flag or ready`. The same static
truth rule feeds satisfy repair ranking for valid hole predicates.
It also evaluates small boolean formulas over up to ten unknown pure
predicates after literal and comparison folding.
It also proves partial case-split `or` predicates with shorter branches that
cover every assignment across up to eight non-static predicates.
Negated partial case-split `and` predicates are also statically proven when
their disjunctive branches reject every assignment for the same predicate set.
It also proves top-level `or` implications where a negated conjunction of
ordering bounds transitively guarantees another ordering bound, such as
`not (low <= mid and mid < high) or low < high`.

## Skip Unless Needed

- Skip the former combined detail unless you need exact rules or examples.
- Use [source-surface.md](source-surface.md#contract-predicates) for the source
  grammar surface before reading older decision records.
