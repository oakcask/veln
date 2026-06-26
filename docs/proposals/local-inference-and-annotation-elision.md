# Local Inference And Annotation Elision

Status: proposed

This proposal defines planned local type-inference improvements that reduce
annotation noise inside function bodies and private helpers without changing
Veln's public-boundary rule. Current implemented behavior remains specified in
`../specification/types.md`.

## Problem

Veln examples often write local `let` annotations only to guide the checker:

- intermediate collection literals need an element type
- `None`, `Nil`, and similar constructors need surrounding context
- private helper parameters or returns repeat types already forced by call
  sites
- callbacks passed to prelude helpers lose concrete argument types and surface
  as `unknown`

Those annotations make examples longer and distract from the source behavior
the example is meant to show. Public signatures should stay explicit, but local
implementation code should not need redundant annotations when the checker can
derive one concrete type from nearby facts.

## Goals

- Let examples remove needless local `let` annotations when a single concrete
  type is forced by local code.
- Keep inference local, monomorphic, and predictable.
- Preserve explicit public function parameter, return, and effect boundaries.
- Keep diagnostics attached to the specific failed fact, with related notes for
  provenance and conflicting constraints.
- Improve typed-hole, repair-candidate, and editor context by exposing more
  concrete expected types.

## Non-Goals

- Do not add generalized let-polymorphism.
- Do not infer public or exported function signatures.
- Do not synthesize anonymous union types for mixed errors.
- Do not add traits, type classes, implicit conversions, or higher-kinded type
  parameters.
- Do not make one local binding usable at multiple unrelated concrete types.
- Do not infer types across module boundaries except through already declared
  public signatures.

## Inference Model

The checker should treat each omitted local fact as a monomorphic inference
slot. A slot may collect constraints from its initializer, annotations around
it, later same-function uses, private helper bodies, and concrete call sites.
The slot succeeds only when all collected facts resolve to one concrete type.

`unknown` remains an internal placeholder while facts are incomplete. A
finished user-facing inferred type must be concrete. If facts conflict or no
concrete type is forced, the checker reports an inference diagnostic instead of
choosing a broad or synthetic type.

Constraint collection is local to a function body or to a private helper and
its same-module concrete call sites. The proposal does not require whole-program
generalization.

## Local Let Annotation Elision

Implemented current behavior is specified in
`../specification/types.md#read-first` and
`../specification/types-full.md#inference` for the narrow same-function local
`let` slice: a binding whose initializer leaves `unknown` may be fixed by a
later call argument or return-compatible use that requires one concrete type.
Unconstrained bindings and later incompatible uses are diagnostics.

Remaining planned work in this section extends omitted local annotations into
broader expected-type paths that are not covered by the current same-function
local `let` and empty collection expected-type slices, including non-empty
collection element inference, typed hole context, and other nested initializer
positions.

Record and nested pattern lets may also omit annotations in a future slice when
the right-hand side has a known record or ADT type and every named binding
receives a concrete field or payload type. Wildcard lets follow the same
checking rule but do not add a binding.

## Completed Private Helper Call-Site Slice

The completed private helper call-site inference slice is archived under
`../reference/implemented-proposals/local-inference-private-helper-call-site.md`.
Current behavior is specified in `../specification/types.md#read-first` and
`../specification/types-full.md#inference`.

## Callback Argument Inference For Prelude Helpers

Prelude higher-order helpers should push their concrete element, value, key,
success, and error types into callback parameters.

Examples of intended sources:

- `vec_map`, `vec_filter`, `vec_fold`, and `vec_try_map`
- `list_map`, `list_filter`, `list_fold`, and `list_try_map`
- `option_map`, `option_then`, `result_map`, `result_map_error`, and
  `result_then`
- dictionary helpers that pass keys or values to callbacks

When a helper input has type `Vec<Int>`, a callback parameter expected to
receive the item should be checked as `Int`, not `unknown`. When the helper
result has an expected type such as `Vec<String>` or `Result<List<String>, E>`,
that expected result may also constrain non-empty callback return types.
The implemented empty collection callback return slice is specified in
`../specification/types-full.md#inference`.

This rule applies only to helpers whose signatures are compiler-known or
declared with enough concrete function type information. It does not invent a
generic function system for ordinary user-defined helpers.

## Bidirectional ADT Constructor Inference

Constructors with payloads should infer type arguments from both expected type
and payload expressions.

Rules:

- If an expected ADT type is available, constructor payloads are checked against
  the instantiated payload types.
- If no expected ADT type is available, payload expression types may determine
  omitted type arguments when the constructor name resolves to one visible
  variant and every type argument becomes concrete.
- For generic constructors with multiple payloads, all payload-derived facts
  must agree on shared type parameters.
- Qualified, import-alias-qualified, and type-qualified constructor forms
  follow the existing visibility and descriptor rules.
- Nullary generic constructors still require surrounding context; payload-free
  constructors do not create fresh public type variables.

Ambiguous constructor names, hidden constructors, and payload facts that do not
determine all type arguments remain diagnostics rather than implicit guesses.

## Match Scrutinee Inference From Constructor Patterns

When a `match` scrutinee has unknown type, constructor patterns in the arms may
constrain the scrutinee type if they identify exactly one finite domain.

Rules:

- The checker first gathers constructor-pattern candidates from visible arms.
- If all constructor patterns point to the same `Option`, `Result`, `List`, or
  source-declared ADT descriptor, that descriptor may constrain the scrutinee.
- Payload subpatterns then receive descriptor payload types, subject to any
  inferred type arguments.
- If multiple descriptors remain possible, the scrutinee type stays unknown and
  the checker reports an ambiguity when a concrete type is required.
- Exhaustiveness still uses the full finite domain after the scrutinee type is
  known, including hidden source-declared constructors in importing modules.
- A catch-all arm alone does not infer the scrutinee type.

This rule improves local pattern examples without making constructor names
global type assertions.

## Empty Collection Literal Inference

Implemented current behavior is specified in
`../specification/types.md#read-first` and
`../specification/types-full.md#inference` for empty `Vec<T>` literals, `Nil`
for `List<T>`, and empty dictionary literals in local annotations, inferred
local binding slots, return positions, call arguments, record fields, match arm
results, constructor payloads, and compiler-known prelude helper result
context for callback return values.

An empty list literal may infer `Vec<T>` when its context expects `Vec<T>`.
`Nil` may infer `List<T>` when its context expects `List<T>`. Empty dictionary
literals may infer `Dict<K, V>` when both key and value types are known from
context.

When no context fixes the element, key, or value types, the checker should keep
the current ambiguity behavior and ask for an annotation rather than rendering
`Vec<unknown>`, `List<unknown>`, or `Dict<unknown, unknown>` as a successful
local type.

## Diagnostics

Diagnostics should distinguish three failure modes:

- `unconstrained`: no local fact determines a concrete type
- `conflicting`: two or more local facts require incompatible concrete types
- `ambiguous`: several declarations, constructors, or finite domains remain
  possible

The primary message should state the specific failed fact at the reported span,
such as an omitted binding type, a callback argument type, a constructor type
argument, or a match scrutinee type. Related notes should carry provenance:
initializer type, later use, earlier call site, helper body fact, expected
return type, or constructor descriptor.

JSON details should include stable fields for the inferred slot kind, current
inferred type when any, and constraint provenance.

The diagnostic-id policy is decided: inference failures should use the narrow
stable id for the slot or failed fact rather than collapsing all omitted local
facts into `type.private_inference_incomplete`. Implemented same-function local
binding gaps use `type.local_inference_incomplete`, private helper signature
gaps use `type.private_inference_incomplete`, ambiguous constructor or literal
contexts use `type.inference_ambiguous`, and conflicting concrete facts use
`type.mismatch` at the incompatible use. Future inference slices should reuse
those ids when their failure shape matches; add a new id only when callers need
to distinguish a new slot kind or ambiguity boundary from the existing local,
private, ambiguous, or mismatch cases.

## Examples Acceptance

Implementation should include an examples cleanup pass. The pass should remove
local `let` annotations in `../../examples/specification/` only when the
program still checks or runs through the existing case harness.

Annotations should remain when they document a public boundary, make a
specification example intentionally explicit, disambiguate a genuinely
ambiguous expression, or test annotation syntax and diagnostics.

Acceptance evidence should include:

- checked examples that use omitted local annotations for empty collections,
  ADT constructors, and prelude callbacks
- negative examples for unconstrained, conflicting, and ambiguous inference
  failures
- human and JSON diagnostic coverage when related provenance is required
- updated specification text under `../specification/` only after behavior is
  implemented

## Implementation Order

1. Infer local `let` bindings from initializers and existing expected-type
   paths.
2. Add empty collection and nullary constructor context propagation.
3. Push concrete prelude helper input and result types into callback
   parameters and returns.
4. Infer payload-carrying ADT constructor type arguments from payloads when the
   constructor descriptor is unambiguous.
5. Infer match scrutinee descriptors from constructor-pattern arms.
6. Run the examples cleanup and keep only annotations that still carry useful
   meaning.

The order is deliberately incremental. Each step should be useful on its own
and should preserve the monomorphic boundary before the next source of
constraints is added.

## Open Questions

- Whether callback literals need dedicated source syntax before callback
  inference can remove enough annotations from examples.
