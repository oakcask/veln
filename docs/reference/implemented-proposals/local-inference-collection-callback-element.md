---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Collection Callback Element


This record keeps the completed collection callback element inference slice
after the behavior moved into the specification and executable examples. It
is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful collection element callback coverage:
  `../../../examples/specification/check/collection-callback-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/collection-callback-inference-diagnostics/`.

## Implemented Boundary

When a concrete expected collection type reaches an element position whose
element type is a concrete function type, a named same-module private callback
function value placed at that element position receives the expected function
parameter types for omitted callback parameter annotations.

The implemented paths cover `Vec<fn(...) -> ...>` literal elements, one
direct local binding hop to a private callback used as a `Vec` literal
element, `List<fn(...) -> ...>` `Cons` head payloads, and nested initializer
positions where an outer concrete record field or constructor payload expected
type reaches one of those collection elements.

The callback return still has to satisfy the element function return type.
When that return type is concrete, it flows into non-empty callback tail
expressions such as `Some(...)`, `Ok(...)`, `Err(...)`, source ADT
constructors, record literals, and collection literals.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported callback definitions are not inferred.
- Anonymous callback syntax remains outside this slice.
- Collection element function types whose parameter or return type still
  contains `unknown` do not constrain callback parameters.
- This slice does not add generalized let-polymorphism, exported alias
  inference, arbitrary collection-builder inference, or a generic function
  system.

## Completion Evidence

- Executable specification examples cover successful direct `Vec`, return
  position, local binding, one direct local callback binding hop, `List`
  `Cons`, and nested initializer collection element callback inference.
- Negative executable examples cover conflicting callback body facts and
  unconstrained or non-concrete collection element paths that keep callback
  parameters unknown.
- Semantic tests check the lowered callback parameter and return types for
  concrete collection element expected-type paths.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this collection callback element
  inference slice is no longer listed as future proposal work.
