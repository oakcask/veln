# Discussion Result: Contract Predicate Parsing

Status: implemented

## Picked Question

- Should contract clauses be parsed as full expressions and rejected by
  contract validation, or parsed through a narrower predicate production from
  the start?

## Decision

Parse `require`, `ensure`, and hole `satisfy` clauses through a narrow contract
predicate production from the start.

The contract predicate grammar is a syntactic subset of ordinary Veln
expressions. It shares token spelling, operator precedence, names, field
access, literals, grouping, and call syntax where those forms are valid in
contracts.

The parser rejects forms that contracts do not accept, including holes, `?`
propagation, pipelines, `match`, records, and lists. Semantic validation still
owns name resolution, type checking, purity, effect checking, and obligation
classification.

## First-Slice Rules

- `ContractPredicate` accepts literals, names, qualified names, grouping, field
  access, plain or qualified call syntax, prefix `not` and `-`, arithmetic,
  comparisons, equality, `and`, and `or`.
- Unsupported predicate syntax in `require` or `ensure` reports
  `parse.contract_predicate`.
- Unsupported predicate syntax in a hole `satisfy` suffix reports
  `parse.satisfy_predicate`.
- A syntactically valid predicate can still fail semantic validation, for
  example because it calls an effectful function or does not produce `Bool`.

## Consequence

The parser gives contract-aware recovery before semantic checking. Veln keeps
contract notation expression-like while making the implementation pipeline
clear: parse only predicate-shaped syntax, then validate semantic admissibility.
