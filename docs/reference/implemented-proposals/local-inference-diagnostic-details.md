# Local Inference Diagnostic Details

Status: implemented

This record keeps the completed local inference diagnostic-detail slice after
the behavior moved into the specification and checked examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current diagnostic JSON details:
  [../../specification/diagnostics-json-full.md#type-inference-diagnostics](../../specification/diagnostics-json-full.md#type-inference-diagnostics).
- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Checked diagnostic examples:
  `../../../examples/specification/check/local-let-inference-diagnostics/`,
  `../../../examples/specification/check/private-helper-inference-diagnostics/`,
  `../../../examples/specification/check/declared-helper-callback-inference-unsupported/`,
  `../../../examples/specification/check/collection-callback-inference-diagnostics/`,
  `../../../examples/specification/check/adt-constructor-inference-diagnostics/`,
  and
  `../../../examples/specification/check/match-scrutinee-inference-diagnostics/`.

## Implemented Boundary

Local inference failure diagnostics expose stable JSON details for the failed
slot and the current inferred type when the checker has one. Local binding
inference gaps use `type.local_inference_incomplete`; private helper signature
gaps use `type.private_inference_incomplete`; constructor, empty collection,
and match scrutinee ambiguity use `type.inference_ambiguous`.

The details identify local binding, private parameter, private return,
constructor type, empty collection, and match scrutinee slots. Ambiguity
diagnostics also expose the known constraint provenance for the ambiguity
boundary.

## Completion Evidence

- Checked examples pin `slot_kind`, `inferred_type`, `missing_fact`, and
  constraint fields for the local inference diagnostic ids.
- The diagnostic implementation reports partially inferred private parameter
  types instead of collapsing them to `unknown`.
- Specification pages describe the stable diagnostic details without relying
  on proposal text as current behavior.

## Skip Unless Needed

- Do not read this page for current diagnostic JSON behavior.
- Use this record only when auditing why the diagnostic-detail slice is no
  longer listed as planned proposal work.
