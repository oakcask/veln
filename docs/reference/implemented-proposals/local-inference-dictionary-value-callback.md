# Local Inference Dictionary Value Callback

Status: implemented

This record keeps the completed dictionary value callback inference slice
after the behavior moved into the specification and executable examples. It
is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful dictionary value callback coverage:
  `../../../examples/specification/check/dictionary-value-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/dictionary-value-callback-inference-diagnostics/`.

## Implemented Boundary

When a concrete expected `Dict<K, fn(...) -> R>` value type reaches a
dictionary literal value position, a named same-module private callback
function value placed at that value position receives the expected function
parameter types for omitted callback parameter annotations.

The implemented paths cover direct dictionary literal values, return-position
or local annotation context, one direct local binding hop to a private
callback used as the dictionary value, and nested initializer positions where
an outer concrete record field or constructor payload expected type reaches
the dictionary value.

The callback return still has to satisfy the value function return type. When
that return type is concrete, it flows into non-empty callback tail
expressions such as `Some(...)`, `Ok(...)`, `Err(...)`, source ADT
constructors, record literals, and collection literals.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported callback definitions are not inferred.
- Anonymous callback syntax remains outside this slice.
- Dictionary key callback inference remains outside this slice.
- Dictionary value function types whose parameter or return type still
  contains `unknown` do not constrain callback parameters.
- This slice does not add generalized let-polymorphism, exported alias
  inference, arbitrary dictionary-builder inference, or a generic function
  system.

## Completion Evidence

- Executable specification examples cover successful direct dictionary value,
  return-position, local annotation, one direct local callback binding hop, and
  nested initializer dictionary value callback inference.
- Negative executable examples cover unconstrained dictionary literals and
  non-concrete dictionary value function contexts that keep callback
  parameters unknown.
- Semantic tests check the lowered callback parameter and return types for
  concrete dictionary value expected-type paths.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this dictionary value callback
  inference slice is no longer listed as future proposal work.
