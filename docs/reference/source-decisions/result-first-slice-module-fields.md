# Discussion Result: First-Slice Module Fields

Status: implemented

## Picked Question

- Which module fields are required in the first slice: purpose, dependencies,
  public API, effects, invariants, examples, tests, decisions?

## Decision

Require only machine-checkable boundary fields in first-slice source modules:
module identity, imports or module dependencies, public API boundaries, and the
effect declarations already required on public functions.

Do not require prose purpose, invariants, examples, tests, or decisions as
first-slice module fields. They should be supported as optional structured
documentation, contracts, doctests, or ADR-lite records, but a valid module
should not need them before the checker can parse, typecheck, report public
effects, and expose JSON diagnostics.

Small single-file programs still need no explicit module header unless they
use package structure, exports, cross-file imports, generated docs, or
non-default tooling. When a source file declares a module, the first slice
should require enough information to make the module boundary explicit and
diagnosable, not enough prose to describe the whole design history.

## Rationale

Parnas's modularity criterion supports keeping design-decision boundaries
close to the code, but it does not imply that every module must carry a
required natural-language design essay. For Veln, the first checker should
capture the boundary facts that can directly shorten repair loops: what module
this is, what it imports, what it exposes, and what public side effects its
callers must account for.

DeRemer and Kron's programming-in-the-large distinction argues for making
inter-module composition explicit. Required imports and public API boundaries
give agents and tools enough structure to reason about cross-module edits,
diagnostics, and later dependency graphs. The earlier module-metadata decision
puts package identity and external dependencies in a manifest, so source-level
module dependencies should mean language imports or required sibling modules,
not package-manager requirements.

The Java module-system precedent reinforces that visibility relationships such
as `requires` and `exports` are semantic module facts, while version selection
and artifact policy belong to tooling. Veln does not need to copy that surface
syntax, but it should adopt the split: the first source-level module form
should make dependency and public-surface facts explicit enough for the
compiler to validate and for diagnostics to locate.

Dependency-network research is a warning against treating manifest metadata as
the whole dependency story. Prazi shows that package-level dependency data can
overapproximate actual source use and that call or source evidence gives a
more precise picture. Therefore a Veln module should expose source-level
imports early, while package-level dependency declarations remain manifest
owned.

Contracts and invariant-oriented systems such as JML and Dafny show that
invariants are useful when they are checked as specifications, not when they
are mandatory decorative fields. Veln already has contract decisions for
`require`, `ensure`, and future `invariant` clauses. Making module invariants
optional in the first slice keeps the checker small while preserving a path to
stronger module-level specifications once examples show the useful grammar.

Purpose statements, examples, tests, and ADR-lite decisions are valuable for
human and agent context, but they are weaker as universal syntactic
requirements. Required prose fields are hard to validate, easy to drift, and
can become generated boilerplate. The first slice should encourage structured
documentation through warnings, docs generation, or project policy later, but
the language core should require only fields that the compiler can check and
use in diagnostics.

## First-Slice Rule

- A source file may omit an explicit module declaration when it is a small
  single-file program with no package structure, exports, cross-file imports,
  generated docs, or non-default tool configuration.
- When a module declaration is present, it must provide a stable module
  identity.
- A module with cross-file dependencies must declare its source-level imports
  or module dependencies in source. External package dependencies remain
  manifest owned.
- The public API boundary must be explicit through public declarations,
  export lists, or an equivalent first-slice mechanism. The exact syntax is
  still open.
- Public functions in the module must follow the existing public-boundary
  rules for explicit parameter types, return types, and coarse effects.
- Module purpose is optional in the language core. A formatter, docs tool, or
  project profile may later warn when public modules lack a short purpose, but
  `veln check` should not reject a module only for missing prose purpose.
- Module invariants are optional until the contract grammar grows a
  module-level invariant form. When present, they must use the same restricted
  specification-expression discipline as other contracts.
- Examples and tests are optional. When present, they should be linked to
  diagnostics and affected-test selection, but absence is not a module-shape
  error.
- ADR-lite decisions are optional documentation records, not required module
  syntax in the first slice.
- JSON diagnostics for missing required module fields should use a stable
  `kind: "module"` envelope and report the missing field, expected owner
  (`source` or `manifest`), module span when available, and a repair hint.

## Remaining Extensions

The implemented first slice resolves source module declarations as `mod`,
source imports as `use`, and public declarations as the public API boundary.
Dedicated export-list syntax and optional purpose text remain future
extensions.

The project may later define stricter documentation profiles for published
packages. That should be a tool or package policy layered over the first-slice
language core, not the minimum validity rule for all modules.

The relationship between module-level examples and doctest execution remains
open and should be handled by the separate doctest question.

## References

- Parnas, D. L. (1972). On the Criteria To Be Used in Decomposing Systems
  into Modules. *Communications of the ACM*, 15(12), 1053-1058.
  https://doi.org/10.1145/361598.361623
- DeRemer, F., & Kron, H. H. (1976). Programming-in-the-Large Versus
  Programming-in-the-Small. *IEEE Transactions on Software Engineering*, 2(2),
  80-86. https://doi.org/10.1109/TSE.1976.233534
- Reinhold, M. (2015). *The State of the Module System*. OpenJDK Project
  Jigsaw. https://openjdk.org/projects/jigsaw/spec/sotms/2015-09-08
- Hejderup, J., Beller, M., Triantafyllou, K., & Gousios, G. (2021).
  *Prazi: From Package-based to Call-based Dependency Networks*.
  arXiv:2101.09563. https://arxiv.org/abs/2101.09563
- Leavens, G. T., Cheon, Y., Clifton, C., Ruby, C., & Cok, D. (2013).
  *JML Reference Manual: Introduction*.
  https://www.cs.ucf.edu/~leavens/JML/jmlrefman/jmlrefman_1.html
- The dafny-lang community. (2026). *Dafny Reference Manual*.
  https://dafny.org/dafny/DafnyRef/DafnyRef

## Consequence

The first slice gets module boundaries that are useful to the checker and to
agents without turning every module into a documentation ceremony. Future docs,
doctest, invariant, and ADR features can add richer context without changing
the minimum module validity rule.
