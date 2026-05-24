# Discussion Result: AST Phase Boundary

Status: implemented

## Picked Question

- What minimal AST shape and phase boundary should the first implementation use
  for holes, contracts, effects, and public boundaries?

## Decision

Use a small source-backed surface AST plus phase-specific analysis tables keyed
by stable node IDs. Do not build separate feature-specific ASTs for holes,
contracts, effects, or public declarations.

The first implementation should have three representation layers:

1. A lossless parse tree for formatting, parse recovery, comments, token spans,
   and syntax-error diagnostics.
2. An untyped source-structured surface AST for `check`, `fmt`, `run`, `test`,
   and JSON diagnostic span anchoring.
3. A checked core view produced only after name, type, contract, effect, and
   reachability analysis have classified the selected program.

The surface AST is the durable boundary between parsing and semantic analysis.
It should preserve source facts: visibility, function signatures, result
bindings, declared effects, contract clauses, expression holes, ordinary
expressions, blocks, imports, and source spans. Semantic facts should be side
tables or overlays, not mutations of the parsed node shape.

## Minimal Surface AST Shape

The first implementation should model these source-backed nodes:

```text
Module {
  id, name?, imports, items, span
}

Item =
  FunctionDecl {
    id, visibility, name, params, return_type, result_binding?,
    effect_decl?, contracts, body, span
  }

ContractClause {
  id, kind: require | ensure, predicate, span
}

Expr =
  Literal | Name | Call | FieldAccess | Pipeline | Record | List | Match |
  ResultPropagation | Hole | ...

Hole {
  id, label?, constraints, span
}
```

`constraints` is present even while the exact `satisfy` surface syntax remains
open. Until that syntax is resolved, parsed holes simply have an empty
constraint list.

Analysis tables should include at least:

- `name_facts`: declarations, references, duplicate-name diagnostics, import
  resolution, and public API membership.
- `type_facts`: inferred and declared types, expected types,
  result-propagation facts, and type-error origins.
- `effect_facts`: declared public effects, inferred direct effects, inferred
  transitive effects, and bounded provenance for missing-effect diagnostics.
- `contract_facts`: validation status, purity status, referenced bindings,
  obligation status, blame side, and runtime-check requirement.
- `hole_facts`: expected type, local bindings, usable contract constraints,
  candidate-query hints, and reachability from selected entry points.
- `boundary_facts`: public signatures, public effects, exported names,
  selected entry points, and test-visible boundaries.

## Phase Rules

- `fmt` uses the lossless parse tree and may consult the surface AST when the
  file parses far enough. It must not require type, contract, or effect
  analysis.
- `check` always builds the surface AST when parsing recovers enough structure,
  then populates analysis tables. Its JSON diagnostics point back to source
  node IDs and spans.
- `run` and `test` require a checked core view for the selected entry point or
  test. They are blocked when parse errors, validation errors, type errors,
  missing required public effects, failed required contracts, or reachable holes
  make the selected execution unsafe or undefined.
- Runtime contract checks are lowered from validated `contract_facts`, not
  reparsed from source text.
- Effect and contract diagnostics should carry provenance slices through
  analysis tables, while the surface AST remains a representation of syntax.
- Public boundary summaries are derived from `FunctionDecl` nodes and
  `boundary_facts`; they are not a separate source language construct.

## Rationale

The main design pressure is to keep partial programs inspectable. Typed-hole
work treats holes as meaningful expressions inside incomplete programs, and
live typed-hole systems rely on the checker being able to expose local context
without requiring a finished program. That supports making `Hole` an ordinary
surface expression and putting expected types, local bindings, and candidate
queries in `hole_facts`.

Contracts and effects need a different separation. Contract systems show that
source clauses can serve static validation, runtime checks, and blame-bearing
diagnostics, while Liquid Types show that stronger static reasoning depends on
restricted, solver-friendly facts rather than arbitrary program execution.
Effect-system work similarly separates declared or inferred effects from value
types. For Veln, the syntax should preserve contract and effect declarations,
but the changing semantic classification belongs in analysis overlays.

AST engineering literature argues against forcing every phase into one node
shape. Najd and Peyton Jones's Trees That Grow work is directly motivated by
the need to decorate compiler syntax trees with phase-specific information in
GHC. Veln does not need the full machinery in the first implementation, but it
should take the same lesson: keep source syntax stable and make phase facts
explicitly attached by node ID.

Diagnostics are also phase products. Program slicing and question-centered
debugging support showing the small slice of evidence relevant to a specific
diagnostic instead of dumping a whole internal graph. That argues for bounded
provenance tables keyed by surface nodes, so `veln check --json` can explain
why a public function needs an effect or why a hole has an expected type without
making the AST itself carry presentation-specific fields.

## First-Slice Rules

- Every source-backed AST node that can appear in a diagnostic has a stable
  `NodeId` and a primary `Span`.
- Parse errors may create error nodes in the parse tree, but unrecovered syntax
  should not be invented as valid surface AST.
- Holes are valid surface expressions. They can block selected execution later,
  but they do not block AST construction or ordinary `check`.
- Contract clauses are source-backed AST nodes even when their predicates later
  fail contract validation.
- Declared public effects are source-backed syntax; inferred private and
  transitive effects are analysis facts.
- Public API membership is derived from visibility and module context, then
  stored in `boundary_facts` for diagnostics and tests.
- Lowering to checked core must not discard source node IDs needed for JSON
  diagnostics, runtime contract failures, or captured test failures.

## Open Details

The first implementation representation is resolved by
[AST Implementation Representation](result-ast-implementation-representation.md):
use arena-allocated source nodes with session-stable `NodeId` handles and
phase-specific side tables, while leaving host-language memory layout and
incremental cache identity for later implementation work.

The exact `satisfy` syntax, kind-specific diagnostic payloads, project model,
runtime contract failure shape, and prelude helper set remain separate open
questions.

## Consequence

The parser, formatter, checker, runtime, and test runner can share one stable
source-backed representation without coupling early syntax decisions to later
semantic experiments. Agents get consistent spans and node identities across
diagnostics, while the implementation remains free to evolve type, contract,
effect, and repair facts behind side-table interfaces.

## References

- Najd, S., & Peyton Jones, S. (2017). Trees That Grow. *Journal of Universal
  Computer Science*, 23(1), 47-62. https://arxiv.org/abs/1610.04799
- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327
- Perelman, D., Gulwani, S., Ball, T., & Grossman, D. (2012). Type-directed
  completion of partial expressions. *PLDI 2012*, 275-286.
  https://doi.org/10.1145/2254064.2254098
- Findler, R. B., & Felleisen, M. (2002). Contracts for higher-order
  functions. *ICFP 2002*, 48-59.
  https://dblp.org/rec/conf/icfp/FindlerF02
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169. https://doi.org/10.1145/1375581.1375602
- Lucassen, J. M., & Gifford, D. K. (1988). Polymorphic effect systems.
  *POPL 1988*. https://doi.org/10.1145/73560.73564
- Weiser, M. (1982). Programmers use slices when debugging. *Communications of
  the ACM*, 25(7), 446-452. https://doi.org/10.1145/358557.358577
