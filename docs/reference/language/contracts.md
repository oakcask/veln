# Contracts

This page routes contract-specific language behavior. It points into the full
combined detail until the contract body is split further.

## Read First

- [Contract clauses and predicate validation](contracts-holes-full.md#contracts)
  defines implemented `require`, `ensure`, and `invariant` clauses.
- [Runtime obligations](contracts-holes-full.md#contracts) defines contract
  enforcement, blame, and static obligation classification.
- [Explicit result bindings](contracts-holes-full.md#contracts) defines when
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
It also proves partial case-split `or` predicates with shorter branches that
cover every assignment across up to eight non-static predicates.

## Skip Unless Needed

- Skip the former combined detail unless you need exact rules or examples.
- Use [source-surface.md](source-surface.md#contract-predicates) for the source
  grammar surface before reading older decision records.
