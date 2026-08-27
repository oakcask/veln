---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Direct Return Callback


This record keeps the completed direct return-position callback inference slice
after the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful direct return callback coverage:
  `../../../examples/specification/check/direct-return-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/direct-return-callback-inference-diagnostics/`.

## Implemented Boundary

A function body tail expression checked against a concrete function return type
can provide expected-type context for a named same-module private callback
function value returned directly from that body. Omitted callback parameter
annotations receive the declared returned function parameter types.

The callback return still has to satisfy the declared returned function return
type, and ordinary function effect assignment keeps pure and effectful callback
compatibility.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Declared returned function types whose parameter or return type still
  contains `unknown` do not constrain callback parameters.
- This slice does not infer through declared helpers, record fields, local
  bindings, constructor payloads, or prelude callback rules beyond their
  already implemented paths.
- This slice does not add anonymous callback literal syntax, generalized
  let-polymorphism, exported alias inference, or a generic function system.

## Completion Evidence

- Executable specification examples cover a direct callback return that infers
  an omitted callback parameter from the concrete returned function type.
- Executable specification examples cover an effectful returned callback when
  the expected function type allows that effect.
- Negative executable examples cover an incompatible direct returned function
  type while preserving the focused `type.mismatch` diagnostic shape.
- Semantic tests check the lowered callback parameter types for pure and
  effectful direct return expected-type paths.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this direct return callback inference
  slice is no longer listed as future proposal work.
