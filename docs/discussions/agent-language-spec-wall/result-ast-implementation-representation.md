# Discussion Result: AST Implementation Representation

## Picked Question

- What concrete first implementation representation should preserve the AST
  phase boundary: enum nodes with side-table maps, arena IDs, or a
  phase-parameterized AST?

## Decision

Use arena-allocated source nodes with stable `NodeId` handles and phase-specific
side tables. Do not use a phase-parameterized AST as the first implementation
representation.

The first implementation should keep these structures separate:

- a lossless parse tree for tokens, comments, formatting, and parse recovery
- a compact surface AST whose nodes are addressed by `NodeId`
- analysis tables keyed by `NodeId` for name, type, contract, effect, hole,
  boundary, and reachability facts
- checked-core data built after semantic analysis, retaining source `NodeId`
  backpointers for diagnostics and runtime failures

`NodeId` values are stable only within one check session and one parsed project
snapshot. They are not serialized as cross-run identities, package ABI, or
long-term cache keys unless a later incremental-compilation decision defines a
separate identity scheme.

## Rationale

The earlier [AST Phase Boundary](result-ast-phase-boundary.md) decision chose a
source-backed surface AST plus phase-specific analysis tables. The remaining
implementation choice is how much of that separation should be encoded in the
type of the AST itself. For the first slice, arena nodes plus side tables give
the checker stable handles and simple ownership without requiring every parser,
formatter, and analysis pass to agree on one extensible typed-tree framework.

Najd and Peyton Jones's *Trees That Grow* is the strongest argument for not
collapsing all phases into one mutable node shape. Their design addresses the
need to decorate syntax trees with phase-specific information while preserving
clear phase structure. Veln should copy the design pressure, but not the whole
abstraction yet: a small language with a fresh implementation gets most of the
benefit by making phase facts explicit side tables keyed by source nodes.

Typed-hole and type-directed-completion research makes stable local context
more important than AST cleverness. Omar, Voysey, Chugh, and Hammer show that
typed-hole tooling depends on exposing expected type, local bindings, and
constraints for incomplete expressions. Perelman, Gulwani, Ball, and Grossman
similarly rely on type-directed local evidence for completion. Veln therefore
needs reliable links from hole syntax to analysis facts; `NodeId` side tables
provide that link without forcing holes, completed expressions, and checked
core terms into one representation.

Diagnostics research pushes the same way. Barik, Ford, Murphy-Hill, and Parnin
argue that compiler explanations need evidence and resolution-oriented context.
For `veln check --json`, the evidence should live in diagnostic details and
analysis provenance, while the surface AST remains source structure. That makes
JSON diagnostics easier to test: the stable envelope points at a span and
`NodeId`, and kind-specific details cite facts from the relevant phase table.

Phase-parameterized ASTs remain a plausible future refactor if the
implementation grows several checked representations. They are too expensive
for the first slice because every early grammar adjustment would also force
generic AST plumbing churn. Arena plus side tables keeps the first parser and
checker smaller while preserving the ability to introduce typed wrappers around
specific tables later.

## First-Slice Rules

- Surface AST nodes are stored in arenas or arena-like collections and are
  referenced by stable `NodeId` handles during one project snapshot.
- `NodeId` handles must be deterministic for a fixed source snapshot and parse
  traversal so golden diagnostics can compare them when useful.
- The surface AST owns source-backed structure: modules, imports, public
  declarations, signatures, contract clauses, effect declarations, blocks,
  expressions, holes, and spans.
- Comments, exact whitespace, and unrecovered parse-error trivia belong to the
  lossless parse tree, not to semantic analysis tables.
- Analysis facts must be stored outside surface nodes. A pass may cache derived
  data, but it must be replaceable from the source AST plus earlier phase
  tables.
- Checked core may use a different representation, but every lowered item that
  can produce a diagnostic, runtime contract failure, or test failure must keep
  a source `NodeId` backpointer.
- Side tables should use typed keys or wrappers per table when practical, but
  the language specification should describe observable behavior, not the host
  language's container types.

## Open Details

This decision does not choose a concrete host implementation language,
incremental cache format, persistent cross-run source identity, memory layout,
or error-node representation. It only fixes the first implementation contract:
source AST nodes have session-stable IDs, and changing semantic facts are
stored in phase tables rather than embedded into source nodes.

## Consequence

The first implementation can start with straightforward parser and checker
data structures while preserving the repair-oriented invariants: diagnostics
can point to stable source nodes, typed holes can collect local context, and
runtime/test failures can retain source provenance. Later implementation work
can optimize the storage model without changing the language-facing AST phase
boundary.

## References

- Najd, S., & Peyton Jones, S. (2017). Trees That Grow. *Journal of Universal
  Computer Science*, 23(1), 47-62. https://arxiv.org/abs/1610.04799
- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327
- Perelman, D., Gulwani, S., Ball, T., & Grossman, D. (2012). Type-directed
  completion of partial expressions. *PLDI 2012*, 275-286.
  https://doi.org/10.1145/2254064.2254098
- Barik, T., Ford, D., Murphy-Hill, E., & Parnin, C. (2018). How Should
  Compilers Explain Problems to Developers? *ESEC/FSE 2018*.
  https://doi.org/10.1145/3236024.3236040
