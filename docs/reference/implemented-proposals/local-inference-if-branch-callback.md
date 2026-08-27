---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference If Branch Callback


This record keeps the completed if-branch callback inference slice after the
behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful if-branch callback coverage:
  `../../../examples/specification/check/if-branch-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/if-branch-callback-inference-diagnostics/`.

## Implemented Boundary

When an `if` expression is checked against a concrete expected function type,
each `then`, `else if`, and final `else` branch result receives that expected
type. A named same-module private callback function value returned from a
branch receives the expected function parameter types for omitted callback
parameter annotations.

This covers a local binding annotation whose initializer is an `if` expression
and a function body tail expression whose declared return type is a concrete
function type. The callback return still has to satisfy the expected function
return type. When that return type is concrete, it flows into non-empty
callback tail expressions such as `Some(...)`, `Ok(...)`, `Err(...)`,
source ADT constructors, record literals, and collection literals.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Expected function types whose parameter or return type still contains
  `unknown` do not constrain callback parameters.
- This slice does not add anonymous callback literal syntax, generalized
  let-polymorphism, exported alias inference, or a generic function system.
- This slice does not broaden callback inference to record fields, constructor
  payloads, helper call arguments, or `match` arms beyond their separately
  specified paths.

## Completion Evidence

- Executable specification examples cover `then`, `else if`, and final `else`
  branch callback parameter inference plus callback return expected-type
  propagation.
- Negative executable examples cover an incompatible callback body fact while
  preserving the focused `type.mismatch` diagnostic shape.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this if-branch callback inference
  slice is no longer listed as future proposal work.
