---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference ADT Constructor Payload


This record keeps the completed payload-carrying ADT constructor inference
slice after the behavior moved into the specification and executable examples.
It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Checked example coverage:
  `../../../examples/specification/check/adt-constructor-inference/`.
- Focused diagnostic coverage:
  `../../../examples/specification/check/adt-constructor-inference-diagnostics/`.

## Implemented Boundary

Payload-carrying ADT constructors infer omitted type arguments from payload
expressions when no surrounding expected ADT type is available. The constructor
name must resolve to exactly one visible variant, and payload-derived facts must
determine every ADT type parameter as one concrete type.

The rule covers compiler-owned `Option`, descriptor-backed `List`, and
source-declared generic ADTs. `Result` constructors without surrounding result
context remain ambiguous unless payloads determine both result type parameters.
Qualified, import-qualified, and type-qualified constructor forms keep the
same descriptor and visibility rules as other constructor calls.

## Boundaries Preserved

- Nullary generic constructors still require surrounding type context.
- Ambiguous constructor names still report the existing name-resolution
  diagnostic rather than selecting a descriptor from payload shape.
- Repeated payload facts for the same type parameter remain monomorphic; a
  conflicting later payload reports `type.mismatch`.
- Match scrutinee constructor-pattern inference was implemented by the later
  [local-inference-match-scrutinee-constructor-pattern.md](local-inference-match-scrutinee-constructor-pattern.md)
  slice.

## Completion Evidence

- Executable specification examples cover successful payload-derived
  constructor inference for built-in and source-declared constructors,
  including qualified forms and imported source constructors.
- Negative executable examples cover unresolved `Result` type arguments and
  conflicting payload-derived facts with `type.inference_ambiguous` and
  `type.mismatch`.
- Semantic tests cover success without expected ADT context, unresolved
  payload-derived constructor type arguments, and conflicting repeated
  constructor type-parameter facts.
- The current specification pages document the inference rule; the remaining
  proposal page keeps only incomplete local-inference work.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why payload-carrying ADT constructor
  inference is no longer listed as future proposal work.
