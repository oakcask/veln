# Local Inference Match Arm Callback

Status: implemented

This record keeps the completed match-arm callback inference slice after the
behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful match-arm callback coverage:
  `../../../examples/specification/check/match-arm-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/match-arm-callback-inference-diagnostics/`.

## Implemented Boundary

When a `match` expression is checked against a concrete expected function type,
each arm result receives that expected type. A named same-module private
callback function value returned from an arm receives the expected function
parameter types for omitted callback parameter annotations.

This covers a local binding annotation whose initializer is a `match`
expression and a function body tail expression whose declared return type is a
concrete function type. The callback return still has to satisfy the expected
function return type. When that return type is concrete, it flows into
non-empty callback tail expressions such as `Some(...)`, `Ok(...)`,
`Err(...)`, source ADT constructors, record literals, and collection literals.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Expected function types whose parameter or return type still contains
  `unknown` do not constrain callback parameters.
- This slice does not add anonymous callback literal syntax, generalized
  let-polymorphism, exported alias inference, or a generic function system.
- This slice does not broaden callback inference to record fields, constructor
  payloads, helper call arguments, or `if` branches beyond their separately
  specified paths.

## Completion Evidence

- Executable specification examples cover match-arm callback parameter
  inference and callback return expected-type propagation in local binding
  initializer and function body tail-expression contexts.
- Negative executable examples cover an incompatible callback body fact while
  preserving the focused `type.mismatch` diagnostic shape.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this match-arm callback inference
  slice is no longer listed as future proposal work.
