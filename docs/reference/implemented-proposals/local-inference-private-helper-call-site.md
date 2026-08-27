---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Private Helper Call-Site

This record keeps the completed private helper call-site inference slice after
the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type annotation and inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Current JSON diagnostic shape:
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md).
- Checked example coverage:
  `../../../examples/specification/check/private-helper-inference/` and
  `../../../examples/specification/check/private-helper-inference-diagnostics/`.

## Implemented Boundary

Private non-exported helper functions may omit parameter and return type
annotations when same-module concrete call sites and body facts determine one
monomorphic signature. Concrete argument expressions constrain omitted
parameters. Concrete expected result context at a helper call constrains an
omitted return type, and the helper body remains checked against the inferred
return type.

The inferred signature is monomorphic. Body facts and call-site facts must
agree; an incompatible later use reports `type.mismatch` at the failed use.
An omitted slot that remains unconstrained or non-concrete reports
`type.private_inference_incomplete`.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit, even though a test body may
  call a private helper and thereby constrain that helper.
- Exported aliases and imported public functions do not receive inferred
  signatures.
- Direct recursive helper calls do not provide inference facts for the helper
  itself; omitted recursive slots still need non-recursive concrete facts or
  annotations.

## Completion Evidence

- Executable specification examples cover a successful private helper whose
  omitted parameter and return types are inferred from same-module call sites,
  including expected result contexts.
- Negative executable examples cover non-concrete and conflicting call-site
  constraints with JSON expectations for `type.private_inference_incomplete`
  and `type.mismatch`.
- Semantic tests cover same-module call-site parameter inference, expected
  call-result return inference, test-body call sites, conflicting call-site
  constraints, and non-concrete call-site constraints.
- The current specification pages document the inference rule and diagnostic
  details; the remaining proposal page keeps only incomplete local-inference
  work.

## Skip Unless Needed

- Do not read this page for current inference or diagnostic rules.
- Use this record only when auditing why private helper call-site inference is
  no longer listed as planned proposal work.
