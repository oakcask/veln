---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Examples Cleanup


This record keeps the completed local inference examples cleanup slice after
the behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Checked local let and empty collection coverage:
  `../../../examples/specification/check/local-let-inference/`.
- Checked constructor, callback, private helper, match scrutinee, and hole
  coverage:
  `../../../examples/specification/check/adt-constructor-inference/`,
  `../../../examples/specification/check/prelude-callback-argument-inference/`,
  `../../../examples/specification/check/declared-helper-callback-inference/`,
  `../../../examples/specification/check/private-helper-inference/`,
  `../../../examples/specification/check/match-scrutinee-inference/`, and
  `../../../examples/specification/check/hole-expected-type-flow-json/`.

## Implemented Boundary

Executable specification examples use omitted local annotations for the
implemented local inference paths: empty collection expected types, ADT
constructor payload inference, compiler-known prelude callback parameters,
declared-helper callback parameters, private helper call-site inference, local
pattern lets, match scrutinee constructor-pattern inference, and typed-hole
expected-type flow.

Annotations remain in examples when they carry public API boundaries,
intentionally demonstrate annotation syntax, disambiguate genuinely ambiguous
source, assert an inferred callback body fact, or anchor a negative diagnostic.

## Completion Evidence

- `local-let-inference` covers omitted local annotations for same-function
  uses, non-empty collection initializer inference, empty collection expected
  types from return, call argument, record field, match arm, `if` branch,
  constructor payload, and prelude callback result contexts, plus record and
  constructor pattern lets.
- `adt-constructor-inference`, `prelude-callback-argument-inference`,
  `declared-helper-callback-inference`, `private-helper-inference`, and
  `match-scrutinee-inference` cover the other implemented annotation-elision
  paths with successful check cases.
- `hole-expected-type-flow-json` and `hole-expected-type-flow-human` cover
  expected-type diagnostics for typed holes, including JSON details and human
  primary messages.
- Diagnostic examples keep unconstrained, conflicting, and ambiguous inference
  failures focused on the failed fact with related provenance where required.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why the examples cleanup slice is no
  longer listed as future proposal work.
