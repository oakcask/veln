# Local Inference Match Scrutinee Constructor Pattern

Status: implemented

This record keeps the completed match scrutinee constructor-pattern inference
slice after the behavior moved into the specification and executable examples.
It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Checked example coverage:
  `../../../examples/specification/check/match-scrutinee-inference/`.
- Focused diagnostic coverage:
  `../../../examples/specification/check/match-scrutinee-inference-diagnostics/`.

## Implemented Boundary

Constructor patterns in `match` arms may constrain an otherwise unknown
scrutinee type when the visible constructor patterns identify exactly one
finite descriptor domain. The rule covers compiler-owned `Option` and
`Result`, descriptor-backed `List`, and source-declared ADTs with bare or
qualified constructor pattern names that are visible at the match site.

Payload literal and nested constructor subpatterns may determine concrete
descriptor type arguments. Once the scrutinee descriptor type is known,
payload bindings receive the descriptor payload types and ordinary
finite-domain exhaustiveness runs over the full descriptor domain.

## Boundaries Preserved

- A catch-all arm alone does not infer the scrutinee type.
- Ambiguous constructor-pattern domains leave the scrutinee unknown and report
  `type.inference_ambiguous` when a concrete scrutinee type is required.
- Imported source-declared ADTs still use the full finite domain for
  exhaustiveness, including hidden constructors.
- Constructor names do not become global type assertions outside the local
  match scrutinee inference boundary.

## Completion Evidence

- Executable specification examples cover successful scrutinee inference for
  `Option`, `Result`, `List`, and a source-declared ADT.
- Negative executable examples cover catch-all-only matches and ambiguous
  constructor-pattern domains with JSON expectations for
  `type.inference_ambiguous`.
- Semantic tests cover the existing descriptor-backed pattern payload binding
  and finite-domain exhaustiveness paths that consume the inferred scrutinee
  type.
- The current specification pages document the inference rule; the remaining
  proposal page keeps only incomplete local-inference work.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why match scrutinee constructor-pattern
  inference is no longer listed as future proposal work.
