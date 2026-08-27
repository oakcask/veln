---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Callback Return Expected Type


This record keeps the completed callback return expected-type slice after the
behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful callback return coverage:
  `../../../examples/specification/check/callback-return-expected-type-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/callback-return-expected-type-inference-diagnostics/`.

## Implemented Boundary

When a concrete helper, record-field, local-binding, or compiler-known prelude
helper context fixes a named private callback return type, that return type
flows into non-empty callback tail expressions whose shape can use the
context. Covered shapes include `Some(...)`, `Ok(...)`, `Err(...)`, source ADT
constructors, record literals, `Vec` literals, and dictionary literals.

The rule applies only when the callback function is private, named, and has an
omitted return annotation. The expected return slot must resolve to one
concrete type. Incompatible payload, field, element, key, or value facts report
the ordinary `type.mismatch` diagnostic at the incompatible expression.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Helper function types whose callback return still contains `unknown` do not
  constrain callback return expressions.
- This slice does not add generalized let-polymorphism, trait inference,
  implicit conversions, or new generic function inference.

## Completion Evidence

- Executable specification examples cover successful expected callback return
  propagation through prelude helpers, same-module and imported declared
  helper calls, record field initializers, local callback bindings, and source
  ADT constructors.
- Negative executable examples cover an incompatible callback payload fact
  reported as `type.mismatch` at the conflicting expression.
- Semantic tests check successful non-empty callback return inference for
  `Some(...)`, `Ok(...)`, records, and collection literals, plus the
  conflicting callback return diagnostic.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this callback return expected-type
  slice is no longer listed as future proposal work.
