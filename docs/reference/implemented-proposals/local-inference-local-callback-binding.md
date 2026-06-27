# Local Inference Local Callback Binding

Status: implemented

This record keeps the completed local callback binding inference slice after
the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful local binding coverage:
  `../../../examples/specification/check/local-callback-binding-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/local-callback-binding-inference-diagnostics/`.

## Implemented Boundary

A local binding annotation whose type is a concrete function type can provide
expected-type context for its initializer. When a named private callback
function value is assigned to that binding, omitted callback parameter
annotations receive the binding function parameter types.

The local binding may then be called in the same function or returned where
the same concrete function type is expected. The callback return still has to
satisfy the binding function return type, and ordinary function effect
assignment keeps pure and effectful callback compatibility.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Local binding function types whose parameter or return type still contains
  `unknown` do not constrain callback parameters.
- This slice does not add anonymous callback literal syntax, generalized
  let-polymorphism, exported alias inference, or a generic function system.

## Completion Evidence

- Executable specification examples cover successful local callback binding
  inference through same-function calls, returns of the binding, and effectful
  callback assignment.
- Negative executable examples cover incompatible callback parameter facts,
  incompatible callback return facts, and a local binding annotation that still
  contains `unknown`.
- Semantic tests check the lowered callback parameter types for pure and
  effectful local binding expected-type paths.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this local callback binding inference
  slice is no longer listed as future proposal work.
