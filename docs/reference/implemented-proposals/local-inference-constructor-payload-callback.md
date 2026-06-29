# Local Inference Constructor Payload Callback

Status: implemented

This record keeps the completed constructor payload callback inference slice
after the behavior moved into the specification and executable examples. It
is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful constructor payload coverage:
  `../../../examples/specification/check/constructor-payload-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/constructor-payload-callback-inference-diagnostics/`.

## Implemented Boundary

When a constructor call is checked against a concrete expected ADT type, each
concrete payload type provides expected-type context for the matching payload
expression. If that payload type is a concrete function type, a named private
callback function value passed at the payload position receives the function
parameter types for omitted callback parameter annotations. This includes
source-declared constructor payloads and compiler-owned bare and
type-qualified `Option` and `Result` payloads.

The callback return still has to satisfy the payload function return type.
When that return type is concrete, it flows into non-empty callback tail
expressions such as `Some(...)`, `Ok(...)`, `Err(...)`, source ADT
constructors, record literals, and collection literals.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported callback definitions are not inferred.
- Anonymous callback syntax remains outside this slice.
- Constructor payload function types whose parameter or return type still
  contains `unknown` do not constrain callback parameters.
- This slice does not add generalized let-polymorphism, exported alias
  inference, or a generic function system.

## Completion Evidence

- Executable specification examples cover successful constructor payload
  callback parameter inference and callback return expected-type propagation.
- Negative executable examples cover conflicting callback body facts and
  unconstrained constructor payloads that keep callback parameters unknown.
- Semantic tests check the lowered callback parameter and return types for a
  concrete constructor payload expected-type path.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this constructor payload callback
  inference slice is no longer listed as future proposal work.
