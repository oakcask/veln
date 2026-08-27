---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Local Callback Binding Annotation Elision


This record keeps the completed omitted local callback binding inference slice
after the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful omitted local callback binding coverage uses the checked example
  named `local-callback-binding-annotation-elision`.
- Diagnostic coverage uses the checked example named
  `local-callback-binding-annotation-elision-diagnostics`.

## Implemented Boundary

When an omitted local binding initializer is a named same-module private
callback function, a later same-function use that expects one concrete function
type propagates that function type through the local binding into the private
callback's omitted parameter and return slots.

The implemented path is one direct local binding hop only. The later expected
type may come from a concrete declared helper callback argument, a
compiler-known prelude callback argument, or a concrete function return
position already supported for local binding values.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Local binding function types whose parameter or return type still contains
  `unknown` do not constrain callback parameters.
- This slice does not add anonymous callback literal syntax, generalized
  let-polymorphism, exported alias inference, multi-hop alias propagation, or a
  generic function system.

## Completion Evidence

- Executable specification examples cover successful omitted local callback
  binding inference through a later concrete helper callback argument.
- Executable specification examples cover callback return expected-type
  propagation through the omitted local binding into `Some(...)` with a
  collection payload.
- Negative executable examples cover an unconstrained omitted local binding
  callback path and conflicting later uses that report `type.mismatch`.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this omitted local callback binding
  inference slice is no longer listed as future proposal work.
