---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference If Branch Local Let


This record keeps the completed if-branch local `let` inference slice after
the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful checked coverage:
  `../../../examples/specification/check/local-let-if-branch-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/local-let-if-branch-inference-diagnostics/`.

## Implemented Boundary

When an omitted local `let` binding is later used as an `if` branch result,
one concrete expected type for the enclosing `if` expression may fix the
binding type. The expected type can arrive from the same concrete paths that
already check direct branch expressions, including declared return types, local
annotations, call arguments, record fields, match arms, outer `if` branches,
and constructor payloads.

The fixed local binding remains monomorphic. After a concrete branch context
fixes the binding, a later incompatible same-function use reports the existing
focused `type.mismatch` diagnostic at the incompatible use.

## Boundaries Preserved

- Public function signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Expected types that still contain `unknown` are not concrete enough to fix
  an omitted local binding.
- Branches that do not force one concrete type keep the existing incomplete or
  ambiguous inference diagnostics instead of widening the binding type.
- This slice does not add generalized let-polymorphism, anonymous function
  syntax, traits, implicit conversions, or cross-module inference.

## Completion Evidence

- Executable specification examples cover declared return, local annotation,
  call argument, record field, match arm, outer `if` branch, and constructor
  payload contexts.
- Executable specification examples cover empty `Vec<T>` literals, `Nil`,
  empty dictionary literals, and simple source-declared constructor
  initializers where direct branch expressions already receive the same
  concrete expected type.
- Negative executable examples cover monomorphic conflicting later uses and
  unknown branch contexts that remain incomplete.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this if-branch local `let` inference
  slice is no longer listed as future proposal work.
