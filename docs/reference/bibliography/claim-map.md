# Bibliography Claim Map

Date: 2026-05-24

## Contract Expression Grammar

- Claim: first-slice contracts should use a pure, side-effect-free boolean
  specification-expression subset rather than arbitrary executable expressions.
- Discussion result:
  [Contract Expression Language](../../discussions/2026-05-24-agent-language-spec-wall/result-contract-expression-language.md)
- Supporting references:
  `meyer1997-object-oriented-software-construction`,
  `findler2002-higher-order-contracts`, `rondon2008-liquid-types`,
  `dafny-reference-manual`, `jml-reference-manual-introduction`.
- Rationale summary: Design by Contract makes preconditions and postconditions
  executable obligations, but static tooling and predictable diagnostics improve
  when specification expressions are pure and solver-friendly. Mature systems
  such as JML and Dafny therefore distinguish specification contexts, pure
  functions, ghost-only constructs, and effectful program actions.

## Contract Static Runtime Boundary

- Claim: first-slice contracts should all be statically validated and exposed
  to diagnostics, while only conservative local obligations are statically
  discharged and valid unknown obligations are enforced at runtime.
- Discussion result:
  [Contract Static Runtime Boundary](../../discussions/2026-05-24-agent-language-spec-wall/result-contract-static-runtime-boundary.md)
- Supporting references:
  `meyer1997-object-oriented-software-construction`,
  `findler2002-higher-order-contracts`, `rondon2008-liquid-types`,
  `freeman1991-refinement-types-ml`,
  `xi1999-dependent-types-practical-programming`,
  `nanevski2006-hoare-type-theory`,
  `swamy2016-fstar-dependent-types-effects`,
  `greenberg2012-contracts-made-manifest`, `dafny-reference-manual`,
  `jml-reference-manual-introduction`.
- Rationale summary: Runtime contract checking gives executable obligations and
  blame metadata early, while Liquid Types, JML, and Dafny show that stronger
  static verification works best when layered over disciplined specification
  subsets instead of assumed for every predicate from the first implementation.
  Refinement, dependent, and Hoare type systems support treating postconditions
  as return-type or computation-type specifications, while manifest and hybrid
  contract systems support checking what is statically known and retaining
  runtime checks for valid obligations that cannot be discharged.

## Postcondition Result Binding

- Claim: postconditions should refer to a returned value through an explicit
  result binding rather than a magic bare `result` identifier.
- Discussion result:
  [Postcondition Result Binding](../../discussions/2026-05-24-agent-language-spec-wall/result-postcondition-result-binding.md)
- Supporting references:
  `meyer1997-object-oriented-software-construction`,
  `jml-reference-manual-introduction`, `dafny-reference-manual`,
  `freeman1991-refinement-types-ml`,
  `xi1999-dependent-types-practical-programming`,
  `nanevski2006-hoare-type-theory`.
- Rationale summary: Design by Contract needs postconditions to describe the
  returned value, but language practice differs on how to name it. JML uses a
  specification-only escaped name, while Dafny supports an explicit result name
  in the signature. Veln should prefer explicit local binding because it avoids
  a global magic identifier and gives agents a semantic repair anchor. The
  explicit binding can also serve as the value variable for future internal
  lowering to return refinements or Hoare-style computation specifications.

## Transitive Effect Diagnostics

- Claim: missing public effects should be reported by coarse effect label with
  bounded provenance slices rather than full transitive call graphs.
- Discussion result:
  [Transitive Effect Diagnostics](../../discussions/2026-05-24-agent-language-spec-wall/result-transitive-effect-diagnostics.md)
- Supporting references:
  `lucassen1988-polymorphic-effect-systems`,
  `talpin1994-type-effect-discipline`,
  `leijen2014-koka-row-polymorphic-effect-types`,
  `weiser1982-programmers-use-slices`, `ko2004-whyline`.
- Rationale summary: Type-and-effect systems support inferring private direct
  and transitive effects, but the repair surface should answer the narrower
  question of why a public boundary gained a missing effect. Program slicing
  and question-centered debugging support displaying a small relevant evidence
  slice with truncation metadata instead of overwhelming users with the full
  internal graph.

## First-Slice Grammar

- Claim: the first Veln slice should use one small line-oriented,
  keyword-delimited, expression-centered grammar instead of multiple equivalent
  surface forms.
- Discussion result:
  [First-Slice Grammar](../../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-grammar.md)
- Supporting references:
  `stefik2013-programming-language-syntax`,
  `lappi2023-syntax-intuitiveness-replication`,
  `qiu2017-syntactic-rule-usage-java`,
  `medeiros2019-peg-error-recovery`,
  `perelman2012-type-directed-completion`, `omar2019-live-typed-holes`.
- Rationale summary: Empirical syntax research supports treating concrete
  syntax as a usability and accuracy factor rather than taste. Syntax-usage
  studies support starting with a restricted grammar and widening only when
  evidence shows pressure. Parser-recovery and typed-hole work support keeping
  partial programs parseable, with holes as ordinary expressions and explicit
  recovery anchors such as `end`.

## Module Metadata Location

- Claim: module metadata should live in both source and a package manifest,
  with package/tool metadata owned by the manifest, compiler-semantic module
  metadata owned by source, and duplicated facts reported as drift.
- Discussion result:
  [Module Metadata Location](../../discussions/2026-05-24-agent-language-spec-wall/result-module-metadata-location.md)
- Supporting references:
  `parnas1972-module-criteria`,
  `deremer1976-programming-large-small`, `cargo-manifest-format`,
  `go-mod-file-reference`, `pypa-pyproject-toml`,
  `openjdk-state-module-system`, `hejderup2021-prazi`.
- Rationale summary: Information-hiding modularity and Java's source-level
  module declarations support keeping semantic boundary facts close to source,
  while Cargo, Go, and Python packaging show that package identity,
  dependencies, toolchain constraints, and publishing metadata belong in a
  machine-readable manifest. Dependency-network research shows that manifest
  data is useful but incomplete without source-level evidence of actual use.

## First-Slice Module Fields

- Claim: first-slice source modules should require only machine-checkable
  boundary fields: module identity, source-level imports or module
  dependencies, public API boundaries, and public function effect declarations.
  Purpose, invariants, examples, tests, and ADR-lite decisions should remain
  optional language-core fields.
- Discussion result:
  [First-Slice Module Fields](../../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-module-fields.md)
- Supporting references:
  `parnas1972-module-criteria`,
  `deremer1976-programming-large-small`,
  `openjdk-state-module-system`, `hejderup2021-prazi`,
  `jml-reference-manual-introduction`, `dafny-reference-manual`.
- Rationale summary: Modularity research supports explicit semantic boundaries
  near source, and module-system practice supports compiler-visible dependency
  and export facts. Dependency-network work cautions that manifest metadata is
  insufficient for actual source use. Contract systems support optional,
  checked invariants when the specification grammar is ready, but required
  prose or examples would add drift-prone boilerplate before the first checker
  can validate them.

## ADR-Lite Decision Location

- Claim: ADR-lite decisions should be optional structured source
  documentation comments attached to modules or public API declarations, with
  generated docs as a derived view rather than canonical language syntax.
- Discussion result:
  [ADR-Lite Decision Location](../../discussions/2026-05-24-agent-language-spec-wall/result-adr-lite-decision-location.md)
- Supporting references:
  `parnas1972-module-criteria`,
  `deremer1976-programming-large-small`,
  `kruchten2009-decision-view`,
  `tyree2005-architecture-decisions`,
  `van-heesch2012-documentation-framework`,
  `knuth1984-literate-programming`,
  `oracle-javadoc-doc-comments`.
- Rationale summary: Architecture-decision research treats rationale as
  first-class architectural knowledge, but Veln's first slice should not turn
  every rationale note into executable language syntax. Source-adjacent
  structured comments preserve locality for repair, allow generated decision
  views, and keep long rationale bodies linkable to separate docs when they
  outgrow source.

## Doctest Result Propagation

- Claim: executable doctest examples should allow `?` only in a generated
  result-returning doctest context, infer the doctest error type only when it
  is local and unambiguous, and otherwise require an explicit doctest error
  type instead of forcing noisy success-value type annotations into examples.
- Discussion result:
  [Doctest Result Propagation](../../discussions/2026-05-24-agent-language-spec-wall/result-doctest-result-propagation.md)
- Supporting references:
  `hoffman2003-api-executable-examples`, `rustdoc-documentation-tests`,
  `python-doctest`, `go-testable-examples`, `knuth1984-literate-programming`.
- Rationale summary: Executable-example research and doctest practice support
  treating examples as readable partial specifications that can be run by the
  toolchain. Rust shows that `?` in documentation examples is useful but needs
  an explicit result-returning context when inference is ambiguous; Veln should
  make that context a doctest-level rule so examples stay copyable and
  diagnostics stay local.

## Minimal Project and Test Discovery

- Claim: `check`, `run`, and `test` should share one first-slice project
  context based on explicit targets, source-relative local imports, explicit
  run entries, and conservative test discovery before manifests and `graph`
  exist.
- Discussion result:
  [Minimal Project and Test Discovery](../../discussions/2026-05-24-agent-language-spec-wall/result-minimal-project-test-discovery.md)
- Supporting references:
  `parnas1972-module-criteria`,
  `deremer1976-programming-large-small`, `hejderup2021-prazi`,
  `rothermel1997-safe-efficient-rts`,
  `rothermel1998-empirical-safe-rts`,
  `graves2001-empirical-rts-techniques`.
- Rationale summary: Modularity and programming-in-the-large work supports
  keeping source imports local while deferring package-scale facts to a
  manifest. Dependency-network research warns that package metadata alone is
  not precise source evidence. Regression-test-selection research supports
  narrowing tests only under explicit assumptions and reporting uncertainty
  when those assumptions are not met.

## AST Phase Boundary

- Claim: the first implementation should use a source-backed surface AST with
  stable node IDs, and store type, contract, effect, hole, public-boundary, and
  diagnostic provenance facts in phase-specific analysis tables.
- Discussion result:
  [AST Phase Boundary](../../discussions/2026-05-24-agent-language-spec-wall/result-ast-phase-boundary.md)
- Supporting references:
  `najd2017-trees-that-grow`, `omar2019-live-typed-holes`,
  `perelman2012-type-directed-completion`,
  `findler2002-higher-order-contracts`, `rondon2008-liquid-types`,
  `lucassen1988-polymorphic-effect-systems`,
  `weiser1982-programmers-use-slices`.
- Rationale summary: Typed-hole and completion work supports representing
  holes as ordinary partial-program expressions. Contract and effect research
  supports separating source declarations from changing semantic
  classifications. Trees That Grow motivates phase-specific AST decoration,
  while program slicing supports bounded provenance facts for diagnostics
  instead of presentation fields embedded in syntax nodes.

## Check JSON Details Fields

- Claim: first-slice `veln check --json` diagnostics should use small
  always-present prototype `details` payloads for parse, type, contract,
  effect, and hole diagnostics, with stable routing facts, expected/actual
  facts, recovery or provenance evidence, and repair context.
- Discussion result:
  [Check JSON Details Fields](../../discussions/2026-05-24-agent-language-spec-wall/result-check-json-details-fields.md)
- Supporting references:
  `lsp-317-diagnostics`, `sarif-210`,
  `barik2018-compiler-explanations`, `medeiros2019-peg-error-recovery`,
  `omar2019-live-typed-holes`, `weiser1982-programmers-use-slices`.
- Rationale summary: LSP and SARIF show that diagnostics need a compact common
  record plus structured extension space. Compiler-error research supports
  evidence and resolution-oriented explanations. Parser recovery, typed-hole,
  and program-slicing work support exposing expected input or types, local
  context, bounded provenance, and deterministic repair-routing facts rather
  than asking agents to scrape prose.

## Safe Repair Candidate Boundary

- Claim: `safe repair` should initially mean a machine-readable, unapplied
  repair candidate with reason, evidence, limits, and verification hints, not
  an automatically applied edit or a correctness guarantee from passing tests.
- Discussion result:
  [Safe Repair Candidate Boundary](../../discussions/2026-05-24-agent-language-spec-wall/result-safe-repair-candidate-boundary.md)
- Supporting references:
  `legoues2012-genprog`,
  `monperrus2018-automatic-software-repair`,
  `qi2015-patch-plausibility-correctness`, `long2016-prophet`,
  `lsp-317-diagnostics`, `sarif-210`.
- Rationale summary: Automatic repair systems can generate useful candidates
  from test-suite oracles, contracts, crash inputs, and learned correctness
  signals, but test-passing plausibility is weaker than semantic correctness.
  Veln should therefore expose candidate edits as structured evidence for the
  repair loop while requiring explicit validation and later user-confirmed or
  gate-confirmed application.

## Scoping and Name Resolution

- Claim: first-slice name resolution should use lexical scope with explicit
  namespaces, reject duplicate declarations in the same scope and namespace,
  resolve nearest lexical value declarations deterministically, and keep
  named-hole labels outside semantic name resolution.
- Discussion result:
  [Scoping and Name Resolution](../../discussions/2026-05-24-agent-language-spec-wall/result-scoping-and-name-resolution.md)
- Supporting references:
  `neron2015-name-resolution`, `van-antwerpen2018-scopes-as-types`,
  `rust-reference-name-resolution`, `barik2018-compiler-explanations`,
  `ghc-typed-holes`.
- Rationale summary: Scope-graph work supports making references,
  declarations, namespaces, and modules explicit semantic facts rather than
  phase-local conventions. Mature language references show why namespaces and
  imports need deterministic ambiguity rules. Compiler-diagnostic research
  supports reporting conflicts with evidence and repair options, while
  typed-hole practice supports treating hole names as diagnostic labels rather
  than ordinary bindings.

## First-Slice Prelude Helpers

- Claim: first examples and golden tests should rely only on a small prelude
  of value-producing list and dictionary update helpers, ordinary list
  traversal helpers, and `Result`/`Option` composition helpers, with
  `list_try_map` as the explicit fallible traversal primitive.
- Discussion result:
  [First-Slice Prelude Helpers](../../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-prelude-helpers.md)
- Supporting references:
  `okasaki1998-persistence`, `mcbride2008-applicative-programming`,
  `haskell-2010-report`, `rust-std-iterator`,
  `rust-std-result-fromiterator`.
- Rationale summary: Persistent data-structure work supports making immutable
  update helpers visibly value-producing. Haskell's standard prelude and list
  library show that a small functional vocabulary is enough for examples
  without method syntax. Applicative traversal gives the general model for
  effectful traversal, while Rust's iterator and `Result` collection behavior
  provide a concrete short-circuiting precedent that fits Veln's first-slice
  `Result` model without introducing type classes.

## Prelude Complexity Guarantees

- Claim: first-slice prelude helper specifications should state value
  semantics, source-order traversal, and `Result` short-circuiting, but should
  not promise asymptotic complexity before Veln chooses concrete persistent
  container representations.
- Discussion result:
  [Prelude Complexity Guarantees](../../discussions/2026-05-24-agent-language-spec-wall/result-prelude-complexity-guarantees.md)
- Supporting references:
  `okasaki1998-persistence`,
  `mcbride2008-applicative-programming`,
  `rust-std-result-fromiterator`.
- Rationale summary: Persistent data-structure research supports immutable
  value semantics but makes clear that costs depend on representation. The
  first-slice prelude should therefore expose correctness-relevant behavior
  that agents need for repair, while leaving performance classes to a later
  representation-backed decision. Applicative traversal and Rust's
  short-circuiting `Result` collection behavior justify specifying
  source-order fallible traversal without implying broader container
  complexity guarantees.

## Comparison Example Task

- Claim: the first multi-language comparison should use one dependency-free
  line-item order summary task that exercises parsing, validation, `Result`,
  collection traversal, tests, stdout, and Veln typed-hole diagnostics without
  claiming general speed, memory, or productivity conclusions.
- Discussion result:
  [Comparison Example Task](../../discussions/2026-05-24-agent-language-spec-wall/result-comparison-example-task.md)
- Supporting references:
  `nanz2015-rosetta-code`, `green1996-cognitive-dimensions`,
  `stefik2013-programming-language-syntax`,
  `lappi2023-syntax-intuitiveness-replication`,
  `barik2018-compiler-explanations`.
- Rationale summary: Rosetta Code research supports using common tasks for
  grounded cross-language comparison, but Veln should keep claims scoped to one
  repair-loop probe. Cognitive Dimensions and empirical syntax work support
  using a complete tiny program rather than isolated syntax snippets, because
  the comparison needs to expose change viscosity, hidden dependencies,
  error-proneness, and diagnostic affordances.
