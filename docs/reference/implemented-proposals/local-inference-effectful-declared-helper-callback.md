# Local Inference Effectful Declared-Helper Callback

Status: implemented

This record keeps the rationale and completion evidence for effectful
declared-helper callback inference. Current behavior is defined by the type
specification and executable examples, not by this historical record.

## Read First

- Current inference summary: [../../specification/types.md](../../specification/types.md).
- Detailed inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Same-module success:
  `../../../examples/specification/check/declared-helper-callback-inference/`.
- Imported-helper and public-alias success:
  `../../../examples/specification/check/declared-helper-callback-import-inference/`
  and
  `../../../examples/specification/check/declared-helper-callback-alias-inference/`.
- Return, effect, and unconstrained-boundary diagnostics:
  `../../../examples/specification/check/declared-helper-effectful-callback-return-diagnostics/`,
  `../../../examples/specification/check/declared-helper-effectful-callback-inference-diagnostics/`,
  and
  `../../../examples/specification/check/declared-helper-effectful-callback-inference-unsupported/`.
- Human-readable unsupported-boundary diagnostic:
  `../../../examples/specification/check/declared-helper-effectful-callback-inference-unsupported-human/`.

## Implemented Boundary

A concrete effectful function parameter on a same-module helper, visible
imported public helper, or resolved public function alias constrains omitted
parameter types on a named same-module private callback passed at that
position. The callback return and effect set remain subject to ordinary
function assignment compatibility, including `type.mismatch` for an
incompatible return or additional effect.

The helper parameter must provide one concrete function type. An undeclared
callback parameter does not become concrete merely because the helper or the
private callback declares effects. Public function signatures remain explicit,
and this slice adds neither effect polymorphism nor cross-module callback
inference.

## Completion Evidence

- Executable examples cover effectful parameter and return inference through
  direct, imported, and public-alias helper calls, including a pure callback
  satisfying an effectful helper parameter.
- JSON diagnostic coverage pins incompatible return and effect types at the
  call-site span.
- Semantic coverage pins direct, imported, and public-alias inference,
  pure-to-effectful compatibility, and return and effect mismatches through
  an imported public helper signature.
- Unsupported-boundary coverage pins the private inference diagnostic when
  the helper's callback parameter remains undeclared despite concrete effect
  declarations elsewhere, including its JSON repair hint and human-readable
  related note.

## Skip Unless Needed

Use this record only to audit why this bounded slice is no longer planned
work. Use the type specification for current language behavior.
