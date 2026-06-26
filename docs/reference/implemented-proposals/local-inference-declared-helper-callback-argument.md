# Local Inference Declared Helper Callback Argument

Status: implemented

This record keeps the completed declared helper callback argument inference
slice after the behavior moved into the specification and executable examples.
It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful same-module coverage:
  `../../../examples/specification/check/declared-helper-callback-inference/`.
- Successful imported-helper coverage:
  `../../../examples/specification/check/declared-helper-callback-import-inference/`.
- Diagnostic and unsupported-boundary coverage:
  `../../../examples/specification/check/declared-helper-callback-inference-diagnostics/`
  and
  `../../../examples/specification/check/declared-helper-callback-inference-unsupported/`.

## Implemented Boundary

Same-module helpers and visible imported public helpers whose declared
parameter type is a concrete function type can provide expected type context
for a named private callback function value passed at that argument position.
The callback receives the declared function parameter types for omitted
callback parameter annotations.

The rule supports fixed function parameter lists and normal function effect
compatibility. Callback returns still have to satisfy the helper's declared
function return type, so incompatible returns and body facts report the
ordinary `type.mismatch` diagnostic at the failed fact.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Helper signatures whose function parameter type still contains `unknown` do
  not constrain callback parameters.
- This slice does not add anonymous callback literal syntax or a generic
  function system.

## Completion Evidence

- Executable specification examples cover successful same-module and imported
  declared helper callback inference.
- Negative executable examples cover incompatible callback body facts and
  unconstrained helper signatures.
- Semantic tests check the lowered callback parameter types for one-argument,
  two-argument, effectful, and imported declared helper calls.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this callback argument inference slice
  is no longer listed as future proposal work.
