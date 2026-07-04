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
- callback paths outside the implemented compiler-known helper slice still
  lose concrete argument types and surface as `unknown`

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
later call argument, return-compatible use, or concrete `if` branch result use
that requires one concrete type.
Non-empty collection literal initializers may also infer omitted local binding
types when every element, key, and value agrees on one concrete type.
Unconstrained bindings and later incompatible uses are diagnostics.

Implemented current behavior also covers record, nested record, constructor,
and nested constructor pattern lets: when the right-hand side or local
annotation has a known record or ADT descriptor type, named bindings receive
their concrete field or payload types. `_` checks the field or payload position
without adding a binding, unconstrained nested bindings report
`type.local_inference_incomplete`, conflicting later facts report
`type.mismatch`, missing fields report `type.field_missing`, and constructors
from the wrong descriptor report `type.mismatch`.

Remaining planned work in this section extends omitted local annotations into
broader inference paths that are not covered by the current same-function
local `let`, local `let` expected-type path, non-empty collection initializer,
empty collection expected-type, hole expected-type flow, nested initializer
expected-type propagation, and local pattern slices.

## Completed Local Let Expected-Type Path Slice

The completed ordinary omitted local `let` expected-type path slice is
archived under
`../reference/implemented-proposals/local-inference-local-let-expected-type-paths.md`.
Current behavior is specified in `../specification/types.md#read-first`,
`../specification/types-full.md#inference`, and the checked examples under
`../../examples/specification/check/local-let-inference/` and
`../../examples/specification/check/local-let-inference-diagnostics/`.

## Completed Hole Expected-Type Flow Slice

The completed typed-hole expected-type flow slice is archived under
`../reference/implemented-proposals/local-inference-hole-expected-type-flow.md`.
Current behavior is specified in `../specification/types.md#read-first`,
`../specification/types-full.md#inference`,
`../specification/holes.md#read-first`,
`../specification/holes-full.md#hole-diagnostics`, and the checked examples
under `../../examples/specification/check/hole-expected-type-flow-json/case.toml`
and
`../../examples/specification/check/hole-expected-type-flow-human/case.toml`.

## Completed Local Pattern Let Slice

The completed local record and constructor pattern `let` slice is archived
under
`../reference/implemented-proposals/local-inference-local-pattern-let.md`.
Current behavior is specified in `../specification/types.md#read-first`,
`../specification/types-full.md#inference`, and the checked examples under
`../../examples/specification/check/local-let-inference/`.

## Completed If Branch Local Let Slice

The completed `if` branch local `let` inference slice is archived under
`../reference/implemented-proposals/local-inference-if-branch-local-let.md`.
Current behavior is specified in `../specification/types.md#read-first`,
`../specification/types-full.md#inference`, and the checked examples under
`../../examples/specification/check/local-let-if-branch-inference/` and
`../../examples/specification/check/local-let-if-branch-inference-diagnostics/`.

## Completed Private Helper Call-Site Slice

The completed private helper call-site inference slice is archived under
`../reference/implemented-proposals/local-inference-private-helper-call-site.md`.
Current behavior is specified in `../specification/types.md#read-first` and
`../specification/types-full.md#inference`.

## Completed Local Nested Initializer Expected-Type Slice

The completed nested initializer expected-type slice is archived under
[local-inference-nested-initializer-expected-type.md](../reference/implemented-proposals/local-inference-nested-initializer-expected-type.md).
Current behavior is specified in `../specification/types.md#read-first`,
`../specification/types-full.md#inference`, and the checked examples under
`../../examples/specification/check/local-let-inference/`.

## Completed Non-Empty Collection Initializer Slice

The completed non-empty collection initializer slice is archived under
[local-inference-non-empty-collection-initializer.md](../reference/implemented-proposals/local-inference-non-empty-collection-initializer.md).
Current behavior is specified in `../specification/types.md#read-first` and
`../specification/types-full.md#inference`.

## Callback Argument Inference For Prelude Helpers

The completed compiler-known prelude callback argument inference slice is
archived under
`../reference/implemented-proposals/local-inference-prelude-callback-argument.md`.
The completed dictionary callback helper alias slice is archived under
`../reference/implemented-proposals/local-inference-dictionary-callback-aliases.md`.
The completed declared helper callback argument slice is archived under
`../reference/implemented-proposals/local-inference-declared-helper-callback-argument.md`.
The completed public member alias boundary for declared helper callback
arguments is archived under
`../reference/implemented-proposals/local-inference-declared-helper-callback-alias.md`.
The completed source-backed prelude callback fallback slice is archived under
`../reference/implemented-proposals/local-inference-prelude-callback-fallback.md`.
The completed record-field callback expected-type slice is archived under
`../reference/implemented-proposals/local-inference-record-field-callback.md`.
The completed local callback binding expected-type slice is archived under
`../reference/implemented-proposals/local-inference-local-callback-binding.md`.
The completed omitted local callback binding annotation-elision slice is
archived under
[local-inference-local-callback-binding-annotation-elision.md](../reference/implemented-proposals/local-inference-local-callback-binding-annotation-elision.md).
The completed direct return-position callback expected-type slice is archived
under
`../reference/implemented-proposals/local-inference-direct-return-callback.md`.
The completed match-arm callback expected-type slice is archived under
`../reference/implemented-proposals/local-inference-match-arm-callback.md`.
The completed if-branch callback expected-type slice is archived under
`../reference/implemented-proposals/local-inference-if-branch-callback.md`.
The completed callback return expected-type slice is archived under
`../reference/implemented-proposals/local-inference-callback-return-expected-type.md`.
The completed constructor payload callback expected-type slice is archived
under
`../reference/implemented-proposals/local-inference-constructor-payload-callback.md`.
The completed collection callback element expected-type slice is archived
under
`../reference/implemented-proposals/local-inference-collection-callback-element.md`.
The completed dictionary value callback expected-type slice is archived under
`../reference/implemented-proposals/local-inference-dictionary-value-callback.md`.
The completed variadic declared-helper callback parameter slice is archived
under
[../reference/implemented-proposals/local-inference-variadic-callback-parameter.md](../reference/implemented-proposals/local-inference-variadic-callback-parameter.md).
Implemented current behavior is specified in
`../specification/types.md#read-first` and
`../specification/types-full.md#inference` for compiler-known `vec_map`,
`vec_filter`, `vec_fold`, `vec_try_map`, `vec_try_map_with`, `list_map`,
`list_filter`,
`list_fold`, `list_try_map`, `dict_map`, `dict_filter`, `dict_fold`,
`dict_try_map`, `dict_map_with`, `dict_filter_with`, `dict_fold_with`,
`dict_try_map_with`, `option_map`, `option_and_then`, `result_map`,
`result_map_err`, and `result_and_then`: concrete helper input types push item,
key, value, success, or error types into named private callback function
parameters. The dictionary `_with` aliases accept a context argument before the
dictionary and pass that context as the first callback argument.
Same-module helpers, visible imported public helpers, and helpers reached
through visible public function aliases whose declared parameter type is a
concrete function type also push that function parameter list into named
private callbacks passed at the matching argument position, including fixed
parameter types and variadic element types for concrete variadic function
types.
Bare and `prelude::` calls to source-backed prelude helpers without a
compiler-known callback rule use the same fallback when the embedded source
signature contains a concrete function-typed callback parameter.
Concrete expected record fields whose type is a concrete function type also
push that function parameter list into named private callbacks placed in the
matching record field initializer. Local bindings whose annotations are
concrete function types also push that function parameter list into named
private callbacks assigned as the binding initializer, and later calls or
returns through the local binding use that concrete function type. Omitted
local bindings whose initializer is a named same-module private callback
function also push one later same-function concrete function expected type
through that direct binding hop into the callback signature. Direct function
body return positions whose declared return type is a concrete function type
also push that function parameter list into named private
callbacks returned from that body. Match arm result expressions checked
against a concrete expected function type also push that function parameter
list into named private callbacks returned from the arm, including local
binding initializer and function body tail-expression contexts. `If` branch
result expressions checked against a concrete expected function type also push
that function parameter list into named private callbacks returned from each
`then`, `else if`, and final `else` branch, including local binding
initializer and function body tail-expression contexts. When those concrete
helper, record-field, local-binding, direct return, match arm, `if` branch,
constructor payload, or prelude helper contexts fix a named private callback
return type, that return type propagates into non-empty callback tail
expressions such as `Some(...)`, `Ok(...)`, `Err(...)`, source ADT
constructors, record literals, and collection literals.
Constructor payloads whose expected type is a concrete function type also push
that function parameter list into named private callbacks passed at the
matching payload position, including compiler-owned bare and type-qualified
`Option` and `Result` payloads.
Concrete `Vec<fn(...) -> ...>` element positions and concrete `List`
`Cons` head positions also push that function parameter list into named
private callbacks passed at that element position, including nested
initializer positions where an outer concrete expected type reaches the
collection element.
Concrete `Dict<K, fn(...) -> ...>` value positions also push that function
parameter list into named private callbacks passed as dictionary values,
including one direct local callback binding hop and nested initializer
positions where an outer concrete expected type reaches the dictionary value.

Remaining planned work in this section covers callback inputs outside the
implemented compiler-known, concrete source-backed prelude signature fallback,
concrete declared-helper signature including visible public function aliases,
concrete record-field expected-type, concrete local-binding expected-type
including omitted direct local callback binding hops, direct return-position
expected-type, concrete match-arm expected-type, concrete if-branch
expected-type, and concrete constructor-payload expected-type, concrete
collection element expected-type, and concrete dictionary value expected-type
paths.

This rule applies only to helpers whose signatures are compiler-known or
declared with enough concrete function type information. It does not invent a
generic function system for ordinary user-defined helpers, and it does not
infer through helper function parameter types that still contain `unknown`.

## Completed Bidirectional ADT Constructor Inference

The completed payload-carrying ADT constructor inference slice is archived
under
`../reference/implemented-proposals/local-inference-adt-constructor-payload.md`.
Implemented current behavior is specified in
`../specification/types.md#read-first` and
`../specification/types-full.md#inference`.

## Completed Match Scrutinee Inference From Constructor Patterns

The completed match scrutinee constructor-pattern inference slice is archived
under
`../reference/implemented-proposals/local-inference-match-scrutinee-constructor-pattern.md`.
Implemented current behavior is specified in
`../specification/types.md#read-first` and
`../specification/types-full.md#inference`.

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

The completed diagnostic-details slice is archived under
`../reference/implemented-proposals/local-inference-diagnostic-details.md`.
Implemented current behavior is specified in
`../specification/diagnostics-json.md`,
`../specification/types.md#read-first`, and
`../specification/types-full.md#inference`: inference failure JSON details
include stable fields for the inferred slot kind, current inferred type when
the checker has one, and constraint provenance when the checker has that
information.

The diagnostic-id policy is decided: inference failures use the narrow stable
id for the slot or failed fact rather than collapsing all omitted local facts
into `type.private_inference_incomplete`. Implemented same-function local
binding gaps use `type.local_inference_incomplete`, private helper signature
gaps use `type.private_inference_incomplete`, ambiguous constructor or literal
contexts and ambiguous match scrutinee domains use `type.inference_ambiguous`,
and conflicting concrete facts use `type.mismatch` at the incompatible use.
Future inference slices should reuse those ids when their failure shape
matches; add a new id only when callers need to distinguish a new slot kind or
ambiguity boundary from the existing local, private, ambiguous, or mismatch
cases.

## Completed Examples Cleanup Evidence

The completed examples cleanup slice is archived under
`../reference/implemented-proposals/local-inference-examples-cleanup.md`.
Current behavior is specified in `../specification/types.md`,
`../specification/types-full.md`, and checked examples under
`../../examples/specification/check/local-let-inference/`,
`../../examples/specification/check/adt-constructor-inference/`,
`../../examples/specification/check/prelude-callback-argument-inference/`,
`../../examples/specification/check/declared-helper-callback-inference/`,
`../../examples/specification/check/private-helper-inference/`,
`../../examples/specification/check/match-scrutinee-inference/`, and
`../../examples/specification/check/hole-expected-type-flow-json/`,
`../../examples/specification/check/direct-return-callback-inference/`, and
`../../examples/specification/check/direct-return-callback-inference-diagnostics/`.

The cleanup keeps annotations when they document a public boundary, make a
specification example intentionally explicit, disambiguate a genuinely
ambiguous expression, or test annotation syntax and diagnostics.

Acceptance evidence includes:

- checked examples that use omitted local annotations for empty collections,
  ADT constructors, prelude callbacks, declared-helper callbacks, private
  helpers, pattern lets, match scrutinees, and hole expected-type paths
- negative examples for unconstrained, conflicting, and ambiguous inference
  failures
- human and JSON diagnostic coverage when related provenance is required
- current specification text under `../specification/`

## Implementation Order

1. Infer local `let` bindings from initializers and existing expected-type
   paths.
2. Completed for empty collection and nullary constructor context propagation,
   including nested record and source-declared constructor initializer
   positions when every enclosing field or payload type is concrete.
3. Completed for current compiler-known collection, dictionary, option, and
   result helpers, including dictionary `_with` aliases, and for same-module
   or visible imported declared helpers, source-backed prelude helper fallback
   signatures with concrete function-typed parameters, concrete record-field
   expected types, concrete local function binding annotations, omitted direct
   local callback binding hops, direct return positions with concrete returned
   function types, concrete match-arm and if-branch expected function types,
   and constructor payload positions with concrete function payload types,
   including compiler-owned `Some`, `Ok`, and `Err`: push concrete helper,
   expected-field, binding, direct-return, arm, branch, or payload input types
   into named private callback parameters, and push concrete callback return
   types into non-empty callback tail expressions.
4. Completed for current payload-carrying constructor calls: infer ADT
   constructor type arguments from payloads when the constructor descriptor is
   unambiguous and every type argument becomes concrete.
5. Completed for current match scrutinee constructor-pattern arms: infer the
   scrutinee finite descriptor when visible constructor patterns identify one
   domain.
6. Completed for the current examples cleanup slice: checked examples use
   omitted local annotations for implemented inference paths, and remaining
   annotations carry public-boundary, explicit syntax, ambiguity, or diagnostic
   meaning.

The order is deliberately incremental. Each step should be useful on its own
and should preserve the monomorphic boundary before the next source of
constraints is added.

## Discussion Result: Callback Literal Syntax

Callback argument inference does not require dedicated callback literal
syntax. The implemented compiler-known prelude helper slice and remaining
planned callback work both apply to existing function values, such as named
private or local functions whose declared function type can be matched against
the helper signature.

Anonymous function syntax remains outside this proposal. If Veln later adds
callback literals, that work should be proposed separately and must define
source syntax, capture behavior, effect checking, formatting, and diagnostics
without changing the callback inference rule chosen here.
