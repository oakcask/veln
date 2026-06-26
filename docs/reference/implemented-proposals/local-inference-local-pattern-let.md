# Local Inference Local Pattern Let

Status: implemented

This record keeps the completed local pattern `let` annotation-elision slice
after the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Current source-surface boundary:
  [../../specification/source-surface-full.md](../../specification/source-surface-full.md).
- Checked example coverage:
  `../../../examples/specification/check/local-let-inference/`.
- Focused diagnostic coverage:
  `../../../examples/specification/check/local-let-inference-diagnostics/`.

## Implemented Boundary

Local record `let` patterns may omit annotations when the right-hand side or
local annotation has a known record type. Named nested bindings receive the
corresponding concrete field type, and `_` checks the field position without
introducing a binding.

Local constructor `let` patterns may omit annotations when the right-hand side
or local annotation has a known ADT descriptor type. Named nested bindings
receive the corresponding concrete constructor payload type, including nested
constructor payload positions.

## Boundaries Preserved

- Pattern bindings whose inferred type still contains `unknown` report
  `type.local_inference_incomplete`.
- Later same-function uses that conflict with a pattern binding report the
  existing `type.mismatch` diagnostic.
- Record pattern fields missing from a known record report
  `type.field_missing` at the pattern field.
- Constructor patterns from the wrong descriptor report `type.mismatch` at the
  constructor pattern.
- Public signatures, imported signatures, generalized let-polymorphism, and
  cross-module pattern inference remain outside this slice.

## Completion Evidence

- Executable specification examples cover accepted record, nested record,
  constructor, and nested constructor pattern inference.
- Negative executable examples cover unconstrained record pattern bindings,
  missing record fields, conflicting constructor pattern bindings, and wrong
  constructor descriptors.
- Semantic tests cover checked core and IR lowering for record and constructor
  pattern bindings plus focused diagnostics for missing fields, unconstrained
  bindings, and wrong constructors.
- The current specification pages document the inference and source-surface
  rule; the remaining proposal page keeps only incomplete local-inference work.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why local pattern `let` inference is no
  longer listed as future proposal work.
