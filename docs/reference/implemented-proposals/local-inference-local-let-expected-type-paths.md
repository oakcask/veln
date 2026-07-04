# Local Inference Local Let Expected Type Paths

Status: implemented

This record keeps the completed ordinary omitted local `let` expected-type
path slice after the behavior moved into the specification and executable
examples. It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful checked coverage:
  `../../../examples/specification/check/local-let-inference/`.
- Diagnostic coverage:
  `../../../examples/specification/check/local-let-inference-diagnostics/`.

## Implemented Boundary

When an ordinary local `let` binding omits its annotation and its initializer
leaves an `unknown` or ambiguous concrete-shape type, a later same-function
use with one concrete expected type may fix the binding.

The concrete expected type may arrive from a declared return, local `let`
annotation, call argument, record field, match arm, `if` branch, constructor
payload, collection element, or dictionary value context. This includes empty
collection literals, `Nil`, empty dictionary literals, and source-declared
nullary constructors whose type can be fixed by the later use.

The fixed local binding remains monomorphic. After one concrete expected type
fixes the binding, a later incompatible same-function use reports the existing
focused `type.mismatch` diagnostic at the incompatible use.

## Boundaries Preserved

- Public function signatures remain explicit.
- Tests, exported aliases, and imported public declarations are not inferred.
- Expected types that still contain `unknown` are not concrete enough to fix
  an omitted local binding.
- Bindings with no concrete same-function use continue to report
  `type.local_inference_incomplete` or the existing ambiguity diagnostic.
- This slice does not add generalized let-polymorphism, traits, implicit
  conversions, callback parameter inference, or cross-module inference.

## Completion Evidence

- Executable specification examples cover successful direct expected-type
  paths for call arguments, local annotations, returns, record fields, match
  arms, constructor payloads, collection elements, and dictionary values.
- Existing executable specification examples cover the `if` branch expected
  type path for omitted local bindings.
- Negative executable examples cover a source-declared nullary constructor
  local fixed by one concrete use and rejected by a later incompatible use.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this ordinary local `let`
  expected-type slice is no longer listed as future proposal work.
