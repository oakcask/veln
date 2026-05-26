# Discussion Result: Hole Satisfy Constraint Grammar

Status: implemented

## Picked Question

- Should `satisfy` constraints on holes share the same expression grammar as
  contracts?

## Decision

Yes. In the first slice, `satisfy` constraints on expression holes should use
the same checked expression subset as `require` and `ensure` contract clauses.

The grammar should be shared at the predicate level, but the surrounding
context is different. A contract predicate is checked against a function
boundary. A hole `satisfy` predicate is checked against one missing expression
and should expose that missing value through an explicit, read-only candidate
binding in the hole-constraint syntax or AST. It must not rely on a magic
identifier such as `result`.

`satisfy` is repair guidance, not a hidden runtime contract and not a full
refinement-type feature. It narrows candidate search, enriches hole diagnostics,
and lets `veln check` reject a proposed fill when the local predicate is
statically false or ill-formed. When the checker cannot discharge the predicate
locally, it should preserve the predicate as an unsatisfied repair constraint
rather than silently accepting it as proof.

## Rationale

Typed-hole research supports treating holes as meaningful partial-program
expressions with type and context, not as comments. Hazelnut and Hazel show
that hole-aware systems can keep incomplete programs analyzable and expose
useful editor/checker information around holes. Veln's `satisfy` predicate
should therefore be part of the typed-hole diagnostic surface, not an unrelated
comment convention.

Type-directed completion work also argues for keeping partial-expression
queries close to ordinary code. Perelman, Gulwani, Ball, and Grossman use
partial expressions with holes as a search interface for likely completions.
For Veln, a small predicate attached to a hole is a natural extension of that
idea: the expected type gives the coarse search shape, while `satisfy` gives a
local semantic filter that can be rendered in JSON diagnostics.

The same decision should not turn hole constraints into a second, more
powerful specification language. The contract-expression decision already
restricts predicates to pure, checkable boolean expressions so diagnostics can
stay local and repairable. Sharing that subset keeps the mental model small:
if a predicate is legal in a contract, the same predicate form is legal as a
hole constraint when it refers to the candidate value and visible pure context.

Liquid Types shows why richer refinement checking can be powerful, but also why
the first slice should be conservative. Solver-friendly refinements work
because the predicate language and inference problem are intentionally
controlled. Veln can later promote common `satisfy` patterns into refinement or
contract machinery, but the first slice should avoid implying that every hole
constraint becomes a globally verified type refinement.

## First-Slice Rule

- `satisfy` predicates use the same pure boolean expression subset as
  `require` and `ensure`.
- A `satisfy` predicate has one explicit candidate binding representing the
  value that will replace the hole. The binding is read-only and scoped only to
  the predicate.
- The predicate may refer to the candidate binding, immutable local bindings in
  scope, constants, fields, records, primitive arithmetic, comparisons, boolean
  connectives, and pure functions accepted by the contract-expression rules.
- The predicate must not perform effects, mutation, allocation, I/O, time,
  randomness, process access, `?` propagation, or general runtime-only work.
- `satisfy` constraints appear in hole diagnostics as repair constraints, with
  source text or structured predicate data under the existing `details`
  payload.
- Filling a hole must typecheck the candidate expression first, then validate
  the `satisfy` predicate in the candidate context.
- If the predicate is syntactically invalid, effectful, or ill-typed, report a
  `kind: "hole"` diagnostic with an id such as `hole.satisfy_invalid`.
- If the predicate is well-formed but cannot be statically discharged, report
  it as an unsatisfied or unknown repair constraint. Do not insert an implicit
  runtime check unless the user writes an explicit contract or assertion.
- Reusing the contract predicate grammar does not decide final surface syntax
  for attaching `satisfy` to holes.

## Open Detail

The exact source syntax for the candidate binding remains open. The important
first-slice requirement is that the binding be explicit enough for diagnostics
and formatting, avoiding a second magic result-name rule after the postcondition
result-binding decision.

The checker still needs a severity policy for unknown `satisfy` predicates.
During ordinary `check`, unknown should probably remain a hint or warning while
the hole is unfilled. For an explicit repair-application command, unknown may
need to block automatic application unless the candidate is otherwise verified
by tests or a contract.

## References

- Omar, C., Voysey, I., Hilton, M., Aldrich, J., & Hammer, M. A. (2017).
  Hazelnut: A bidirectionally typed structure editor calculus. *POPL 2017*,
  86-99. https://doi.org/10.1145/3009837.3009900
- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327
- Perelman, D., Gulwani, S., Ball, T., & Grossman, D. (2012). Type-directed
  completion of partial expressions. *PLDI 2012*, 275-286.
  https://doi.org/10.1145/2254064.2254098
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169. https://doi.org/10.1145/1375581.1375602

## Consequence

Veln gets one predicate language for contracts and hole constraints, which
keeps the first checker and diagnostics small. Agents can use `satisfy`
constraints to rank or reject candidate fills without assuming that hole
constraints are hidden runtime assertions or globally verified refinements.
