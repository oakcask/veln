# Discussion Result: Contract Expression Language

Status: implemented

## Picked Question

- Should `require`, `ensure`, and future `invariant` clauses use arbitrary
  executable expressions or a restricted contract language?

## Decision

Use a restricted contract expression language in the first slice.

Contract clauses use ordinary expression spelling only where the expression is
pure, statically inspectable, and usable as a boolean predicate. Implemented
contracts accept literals, names in scope, field access, equality and ordering,
boolean connectives, primitive arithmetic, and calls to discovered pure
functions.

Contract clauses must not call effectful functions, perform I/O, propagate
`Result` with `?`, use holes, or use runtime-only expression forms such as
records, lists, pipelines, or `match`.

## First-Slice Rules

- `require` and `ensure` clauses must validate as `Bool` predicates.
- Function calls in contract predicates must resolve to discovered functions
  with no effects.
- A pure function call may be used directly when it returns `Bool`.
- A pure function call that returns another type may appear where its result is
  compared or passed as an argument to another pure call.
- Call arguments must be assignable to the declared parameter types.
- Effectful calls, qualified call targets, unresolved calls, arity mismatches,
  and argument type mismatches are contract-language rejections.
- Contract diagnostics distinguish type failures from contract-language
  rejections so repair tooling can decide whether to change the predicate,
  adjust purity or effects, or move logic into a pure helper.

## Consequence

Contracts remain close to source expressions without becoming arbitrary program
execution. The checker can give local diagnostics for contract syntax, type,
purity, and effect violations while leaving runtime contract enforcement and
stronger specification constructs for later slices.
