# Bibliography Source Families

## Contract and Specification Languages

References in this family cover Design by Contract, executable contract
checking, refinement-style static reasoning, and mature specification-language
practice. They also cover the related idea that postconditions can be modeled
as return refinements or Hoare-style computation specifications, while
undischarged but executable obligations remain runtime checks.

- `meyer1997-object-oriented-software-construction`
- `findler2002-higher-order-contracts`
- `rondon2008-liquid-types`
- `freeman1991-refinement-types-ml`
- `xi1999-dependent-types-practical-programming`
- `nanevski2006-hoare-type-theory`
- `swamy2016-fstar-dependent-types-effects`
- `greenberg2012-contracts-made-manifest`
- `dafny-reference-manual`
- `jml-reference-manual-introduction`

## Effect Systems and Diagnostic Provenance

References in this family cover static effect inference, effects as part of a
function interface, and debugging displays that select the evidence relevant to
a specific question instead of showing a whole graph. It also covers diagnostic
interchange formats and empirical work on useful compiler explanations.

- `lucassen1988-polymorphic-effect-systems`
- `talpin1994-type-effect-discipline`
- `leijen2014-koka-row-polymorphic-effect-types`
- `weiser1982-programmers-use-slices`
- `ko2004-whyline`
- `lsp-317-diagnostics`
- `sarif-210`
- `barik2018-compiler-explanations`

## Syntax, Grammar, and Partial Programs

References in this family cover empirical programming-language syntax
usability, real-world syntactic rule usage, parser recovery for invalid source,
name resolution, type-directed completion, live typed-hole workflows, and AST
representation for phase-specific compiler information.

- `neron2015-name-resolution`
- `van-antwerpen2018-scopes-as-types`
- `najd2017-trees-that-grow`
- `stefik2013-programming-language-syntax`
- `lappi2023-syntax-intuitiveness-replication`
- `qiu2017-syntactic-rule-usage-java`
- `medeiros2019-peg-error-recovery`
- `perelman2012-type-directed-completion`
- `omar2019-live-typed-holes`
- `ghc-typed-holes`
- `rust-reference-name-resolution`

## Comparative Language Evaluation

References in this family cover cross-language example corpora, notation
usability dimensions, empirical syntax evaluation, and compiler explanations
used to keep comparison examples scoped to repair-loop evidence.

- `nanz2015-rosetta-code`
- `green1996-cognitive-dimensions`
- `stefik2013-programming-language-syntax`
- `lappi2023-syntax-intuitiveness-replication`
- `barik2018-compiler-explanations`

## Module and Package Metadata

References in this family cover information-hiding modularity, the separation
between programming small source units and programming large compositions,
package manifests, language-level module declarations, and empirical limits of
manifest-only dependency reasoning.

- `parnas1972-module-criteria`
- `deremer1976-programming-large-small`
- `cargo-manifest-format`
- `go-mod-file-reference`
- `pypa-pyproject-toml`
- `openjdk-state-module-system`
- `hejderup2021-prazi`

## Architecture Decisions and Source-Generated Documentation

References in this family cover architectural decision capture, decision
viewpoints, generated documentation from source-adjacent comments, and
literate programming as the heavier historical version of source/documentation
co-location.

- `kruchten2009-decision-view`
- `tyree2005-architecture-decisions`
- `van-heesch2012-documentation-framework`
- `knuth1984-literate-programming`
- `oracle-javadoc-doc-comments`

## Executable Documentation and Doctests

References in this family cover examples that are both human-facing
documentation and machine-executed tests, including expected-output examples,
compile/run doctests, and result-returning doctest wrappers.

- `hoffman2003-api-executable-examples`
- `rustdoc-documentation-tests`
- `python-doctest`
- `go-testable-examples`
- `knuth1984-literate-programming`

## Automatic Repair and Patch Validation

References in this family cover automatic software repair, repair oracles,
test-suite-based validation, patch plausibility versus correctness, and learned
ranking of candidate repairs.

- `legoues2012-genprog`
- `monperrus2018-automatic-software-repair`
- `qi2015-patch-plausibility-correctness`
- `long2016-prophet`

## Regression Test Selection

References in this family cover safe regression test selection, empirical
tradeoffs in selecting subsets of tests after changes, and the limits of
claiming a narrowed test run is complete without sufficient dependency
evidence.

- `rothermel1997-safe-efficient-rts`
- `rothermel1998-empirical-safe-rts`
- `graves2001-empirical-rts-techniques`

## Functional Containers and Traversal

References in this family cover immutable/persistent container updates,
ordinary functional container operations, effectful traversal, and pragmatic
fallible iterator behavior used as precedent for first-slice prelude helpers.

- `okasaki1998-persistence`
- `mcbride2008-applicative-programming`
- `haskell-2010-report`
- `rust-std-iterator`
- `rust-std-result-fromiterator`
