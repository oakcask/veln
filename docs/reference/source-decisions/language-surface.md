# Language Surface Decisions

Read these records only when the categorized language specification needs rationale
or compatibility context.

## Read First

- Current syntax and source grammar:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current type and value behavior: [../../specification/types.md](../../specification/types.md).
- Current contract and hole behavior:
  [../../specification/contracts-holes.md](../../specification/contracts-holes.md).
- Current name, prelude, stdio, and effect behavior:
  [../../specification/names-effects.md](../../specification/names-effects.md).

## Read When

- Use the sections below only after the implemented language page names a
  boundary but does not explain why it exists.
- Open an individual `result-*.md` record only for the selected topic.

## Source Shape

- [Block Structure](records/result-block-structure.md)
- [Compact Function Form](records/result-compact-function-form.md)
- [First-Slice Grammar](records/result-first-slice-grammar.md)
- [First-Slice Module Fields](records/result-first-slice-module-fields.md)
- [Method Call Boundary](records/result-method-call-boundary.md)
- [Pipeline Style](records/result-pipeline-style.md)
- [Test Declaration Syntax](records/result-test-declaration-syntax.md)

## Types And Values

- [Error Type Inference](records/result-error-type-inference.md)
- [First-Slice Value Mutability](records/result-first-slice-value-mutability.md)
- [Public Function Type Boundaries](records/result-public-function-type-boundaries.md)
- [User-Defined ADTs in the First Slice](records/result-user-defined-adts-first-slice.md)

## Contracts And Holes

- [Contract Expression Language](records/result-contract-expression-language.md)
- [Contract Predicate Parsing](records/result-contract-predicate-parsing.md)
- [Contract Static Runtime Boundary](records/result-contract-static-runtime-boundary.md)
- [Hole Satisfy Constraint Grammar](records/result-hole-satisfy-constraint-grammar.md)
- [Hole Satisfy Source Syntax](records/result-hole-satisfy-source-syntax.md)
- [Minimum Type System for Holes](records/result-minimum-type-system-for-holes.md)
- [Named Hole Syntax](records/result-named-hole-syntax.md)
- [Postcondition Result Binding](records/result-postcondition-result-binding.md)

## Names And Effects

- [Effect Access Modes](records/result-effect-access-modes.md)
- [Effect Declaration Boundary](records/result-effect-declaration-boundary.md)
- [First-Slice Prelude Helpers](records/result-first-slice-prelude-helpers.md)
- [One-Shot Resumable Handler Boundary](records/result-one-shot-resumable-handler-boundary.md)
- [Scoping and Name Resolution](records/result-scoping-and-name-resolution.md)

## Skip Unless Needed

Use [../../specification/source-surface.md](../../specification/source-surface.md),
[../../specification/types.md](../../specification/types.md),
[../../specification/contracts-holes.md](../../specification/contracts-holes.md), or
[../../specification/names-effects.md](../../specification/names-effects.md) before opening
these decision records for implemented behavior.
