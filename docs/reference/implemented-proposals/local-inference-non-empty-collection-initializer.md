# Local Inference Non-Empty Collection Initializer

Status: implemented

This record keeps the completed non-empty collection initializer inference
slice after the behavior moved into the specification and executable examples.
It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Checked example coverage:
  `../../../examples/specification/check/local-let-inference/`.
- Focused diagnostic coverage:
  `../../../examples/specification/check/local-let-inference-diagnostics/`.

## Implemented Boundary

An omitted local `let` binding may infer `Vec<T>` from a non-empty vector
literal initializer when the first element determines one concrete element type
and all later elements agree with it.

An omitted local `let` binding may infer `Dict<K, V>` from a non-empty
dictionary literal initializer when the first entry determines concrete key and
value types and all later keys and values agree with them.

## Boundaries Preserved

- Empty `[]`, `Nil`, and `{}` still need concrete expected collection context
  or a later same-function use that fixes the omitted local binding type.
- Local inference remains monomorphic. Conflicting entries report the existing
  `type.mismatch` diagnostic at the incompatible element, key, or value.
- Public signatures, generalized let-polymorphism, callback argument inference,
  private helper inference, ADT constructor inference, and match scrutinee
  inference are unchanged by this slice.

## Completion Evidence

- Executable specification examples cover successful omitted local inference
  from non-empty `Vec<T>` and `Dict<K, V>` literal initializers.
- Negative executable examples cover conflicting vector elements, dictionary
  keys, and dictionary values with focused `type.mismatch` diagnostics.
- Semantic tests cover checked core and IR lowering for successful inferred
  non-empty collection locals and focused diagnostics for conflicting facts.
- The current specification pages document the inference rule; the remaining
  proposal page keeps only incomplete local-inference work.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why non-empty collection initializer
  inference is no longer listed as planned proposal work.
