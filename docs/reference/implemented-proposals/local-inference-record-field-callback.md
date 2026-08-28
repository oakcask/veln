---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Record Field Callback


This record keeps the completed record-field callback inference slice after
the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful record-field coverage:
  `../../../examples/specification/check/record-field-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/record-field-callback-inference-diagnostics/`.

## Implemented Boundary

A concrete expected record type can provide expected-type context for a record
literal field. When that expected field type is a concrete function type, a
named private callback function value in the field initializer receives the
function parameter types for omitted callback parameter annotations.

The callback return still has to satisfy the expected field function return
type. Incompatible callback body facts report the ordinary `type.mismatch`
diagnostic at the record field initializer.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Expected record field types whose function type still contains `unknown` do
  not constrain callback parameters.
- This slice does not add anonymous callback literal syntax, generalized
  let-polymorphism, or a generic function system.

## Completion Evidence

- Executable specification examples cover successful record-field callback
  inference and the incompatible callback body diagnostic.
- Semantic tests check the lowered callback parameter type for the record-field
  expected-type path.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this record-field callback inference
  slice is no longer listed as future proposal work.
