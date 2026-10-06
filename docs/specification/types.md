---
role: specification
authority: normative
specification-coverage: usage=#representative-usage; behavior=#inference-rules; limits=#compatibility-and-limits
update-when: Veln type annotations, inference, assignment compatibility, operator typing, or type-checker behavior changes.
---

# Types

## Representative usage

This source pattern combines a generic constructor, a record annotation,
and a function effect row:

```veln
fn describe(items: Vec<String>) -> Result<{count: Int}, String>
	Ok({count: prelude_builtin::vec_len(items)})
end
```

The expected return type determines the `Ok` constructor type. A nullary generic
constructor without concrete context remains ambiguous.

This file specifies implemented type annotations, inference, assignment
compatibility, and operator typing.

## Usage and annotations

Implemented type annotations:

- primitives: `Bool`, `Int`, `Float`, `String`, and `()`
- built-in and descriptor-backed type constructors: `Option<T>`,
  `Result<T, E>`, `List<T>`, `Vec<T>`, and `Dict<K, V>`
- standard prelude byte and codec vocabulary names: `Byte`, `ByteChunk`,
  `ByteView`, `ByteOffset`, `ByteCount`, `StreamInput`,
  `AcceptOutcome`, `StreamReadOutcome`, `DecodeStep<T>`,
  `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`, and `EncodeError`
- opaque network resource names `NetListener` and `NetStream`, including
  public standard-package aliases that resolve to either resource type
- the standard structural wall-clock name `WallTime`
- records: `{name: Type, ...}`
- function types: `fn(T) -> U`, `fn(T, U) -> V`, or `fn(T, ...U) -> V`
  with optional `effects [name, ...]`
- other named type paths with optional type arguments, unless they are one of
  the arity-checked built-ins above
- a source-defined or compiler-known finite ADT singleton such as
  `State::Ready` or `Option<Int>::Some`, and a same-ADT finite set such as
  `State::Ready | State::Closed`

Angle brackets are the source spelling for type constructor arguments. Legacy
parenthesized type constructor arguments in type positions are invalid type
annotations.

`Option<T>` and `Result<T, E>` are compiler-owned built-in ADTs. `List<T>` and
source-declared ADTs use descriptor entries for constructor payload typing,
qualified and unqualified constructor names, postfix `?` result propagation for
`Result`, and finite-domain exhaustiveness. Source ADTs may be generic and
recursive through variant payloads. Constructor payload types instantiate the
declared type parameters from surrounding context and payload expressions.
Refinement annotations in source-declared payloads resolve against the owning
module like function annotations. An exact singleton payload is accepted, but
the direct-widening rule does not recurse into a payload type.
Nullary generic constructors require surrounding type context; when no
assignment, return, call, match, or other expected type determines the omitted
parameter, inference reports an ambiguous constructor type.

The standard prelude byte vocabulary uses `Byte` for one byte value,
`ByteChunk` for an immutable owned byte sequence, `ByteView` for a bounded
immutable view into byte data, `ByteCount` for byte lengths and consumed or
produced counts, `ByteOffset` for absolute byte offsets, `StreamInput` for
incremental input events, `AcceptOutcome` for adapter-owned listener accept
decisions, `StreamReadOutcome` for adapter-owned stream read decisions,
`StreamWriteOutcome` for adapter-owned stream write decisions, and
`DecodeStep<T>` and `EncodeStep<TState>` for ordinary source-visible codec
boundary values. `StreamInput` is a public ADT with `Chunk(bytes: ByteChunk)`
and `End` variants. A zero-length `ByteChunk` inside `Chunk` remains a chunk
arrival and is not equivalent to `End`. `AcceptOutcome` is a public ADT with
`AcceptStream(stream: NetStream)`, `AcceptEnd`, `AcceptDeadlineExpired`, and
`AcceptCancelled` variants.
`StreamReadOutcome` is a public ADT with `ReadChunk(bytes: ByteChunk)`,
`ReadEnd`, `ReadDeadlineExpired`, and `ReadCancelled` variants.
`StreamWriteOutcome` is a public ADT with `WriteCompleted`,
`WriteDeadlineExpired`, and `WriteCancelled` variants.
`EncodeStep<TState>` is a public ADT with `Encoded`, `Partial`, and `Invalid`
variants; its output payloads use `List<ByteChunk>` and its `Partial` variant
carries the encoder state as `TState`. Prelude helpers also construct and
append outgoing `List<ByteChunk>` values without adding a separate output-only
byte type. `DecodeError` and `EncodeError` are public structured error ADTs for
matching and inspection by ordinary source.
The constructor layout of the other byte vocabulary types is not a public
source contract; programs construct and inspect those values through the
prelude helpers in
[prelude-helpers.md#helper-signatures](prelude-helpers.md#helper-signatures).

In a function or test return annotation, a returned function type may carry its
own effect list before the enclosing declaration's effect list. For example,
`-> fn(String) -> () effects [stdio]` returns a callback that may
perform `stdio` while the factory declaration itself is pure.

A function type parameter may be variadic by writing `...T` as the final
parameter type. The marker is not an ordinary type constructor and is rejected
outside function declaration parameter syntax and function type parameter
syntax. Inside a function body, a variadic declaration parameter is bound as
`List<T>`.

Record type field lists may include a trailing comma, as in
`{name: String, count: Int,}`.

One record type annotation cannot declare the same field name twice. A
duplicate field in a record type annotation is an invalid type annotation.

The standard `WallTime` and `prelude::WallTime` annotations denote the
structural record `{unix_seconds: Int, nanosecond: Int}`. Public type aliases
that target either spelling preserve that record shape in function, local,
effect-operation, and handler annotations. Values of that type are assignable
to anonymous records requiring the same fields, and compatible record literals
are assignable to it. The type checker does not normalize or range-check those
literals; only the `time::wall_time` boundary provides normalized readings. A
source-declared ADT named `WallTime`, and an alias that targets that ADT,
remain nominal and do not acquire the standard record fields.

`NetListener` and `NetStream` are opaque nominal resource types. They have no
source constructors or variants. A public type alias can target either
resource directly, and later public aliases can target that alias. The alias
chain preserves the underlying resource identity; it does not make the host
handle constructible or expose a representation.

Public functions must annotate every parameter and the return type. Their
effect clause must name every inferred effect; a pure declaration omits the
clause. Explicit empty `effects []` declaration clauses are rejected. Private functions may omit a
parameter or return annotation only when local inference produces a concrete
type for the omitted fact. If the checker still has `unknown`, it reports
`type.private_inference_incomplete`. The JSON details identify whether the
missing slot is a private parameter or private return, name private parameters,
report the missing fact, and include the current inferred type.

The optional result binding in `-> name: Type` names the return value for
postconditions, but the type annotation remains `Type`.

Test declarations must use an empty parameter list and annotate the return type
as `()` or `Result<(), E>`. Their effect clause must cover directly inferred
effects; pure tests omit the clause and explicit empty lists are rejected.
Test declarations are not callable function values.

## Inference rules

Local inference is monomorphic and flow-sensitive within one function body.
Expected types flow into holes and subexpressions from:

- declared return types for tail expressions
- local `let` annotations
- function call parameters
- prelude helper parameters and return context
- record fields
- vec elements
- dictionary keys and values
- callable function declarations used as values
- `Ok`, `Err`, `Some`, `None`, `Nil`, `Cons`, source-declared constructors,
  their type-qualified and import-alias-qualified forms, and postfix `?`
- `match` arm results, `if` branch results, and constructor payload bindings
- record pattern field bindings in `match` arms and `let` statements

A `begin` expression checks its body in a nested lexical type environment. Its
final expression supplies the `begin` type and receives the enclosing expected
type. A body with no final expression has type `()`. Bindings declared inside
the body do not escape it. Local annotations in nested `begin` and `defer`
bodies, including a `begin` used as a handler operation clause, receive the
same type-name and schema-boundary checks as local annotations in a function
body. A schema declaration is not an ordinary type, and schema-only primitive
spellings remain invalid in these annotations.

A deferred block must have type `()`. Postfix `?` inside the block reports
`defer.propagation`; a non-unit block reports `defer.non_unit`; and another
`defer` inside it reports `defer.nested`. These restrictions prevent a
deferred block from replacing the cleanup region's value or transferring
result propagation out of the block.

Typed holes use the same concrete expected-type flow as other subexpressions.
When a hole appears under a concrete return, call argument, record field, `if`
branch, `match` arm, or constructor payload context, the hole diagnostic and
JSON details report that type and use it to build advisory symbol candidate
queries.

When a local `let` binding omits its annotation and its initializer leaves the
binding type with `unknown`, later same-function uses may fix the binding to
one concrete type. Implemented constraining uses are declared return
positions, local `let` annotations, call arguments, record fields, match arm
results, `if` branch results, constructor payloads, collection elements, and
dictionary values.
The expected type must be concrete; expected types that still contain
`unknown` are not enough to fix the binding. The binding remains monomorphic:
after one concrete type is fixed, a later incompatible use reports
`type.mismatch`. If no same-function use fixes every `unknown` part of the
binding type, checking reports `type.local_inference_incomplete` at the
omitted binding. Initializers with an ambiguous concrete shape, including
empty collection literals, `Nil`, empty dictionary literals, and nullary
generic constructors, use the existing ambiguity diagnostics until a concrete
same-function expected type fixes the binding. The JSON details identify the
local binding slot and include the current inferred type.

Non-empty `Vec<T>` and `Dict<K, V>` literals infer their element, key, and
value positions from every successfully typed contribution. Constructor
refinements of the same resolved ADT and compatible generic arguments join
into a declaration-ordered variant set, independent of source order. An
unresolved generic argument can become concrete from another contribution;
after it is concrete, later contributions must have the same argument. A join
containing every declared variant, or a join of a refinement and its base ADT,
becomes the base ADT. A contribution that produces its own diagnostic does not
change the accumulated type, so a later successful contribution joins with
only the preceding successful facts. Other conflicting facts remain focused
`type.mismatch` diagnostics at the incompatible element, key, or value.

When a private non-exported helper omits parameter or return annotations,
same-module concrete call sites may constrain the helper's single monomorphic
signature. Concrete argument expressions constrain omitted parameters. A
concrete expected result type at a helper call constrains an omitted return
type, and body tail facts are checked against the inferred return type. Body
facts and call-site facts must agree; a later incompatible call reports
`type.mismatch` at the failed argument or expected-result use. When the body
tail is a resolved constructor, an omitted return retains the constructor
singleton and its resolved generic arguments. It can therefore satisfy a
refined call parameter without an annotation. A compatible function-value
context can instead fix that omitted result to the base ADT before function
compatibility is checked. An ordinary direct call widens the singleton at the
call boundary without replacing the inferred helper signature. Direct recursive
edges do not supply inference facts for the recursive helper itself, so an
omitted recursive slot still needs a non-recursive concrete fact or an
annotation. Public functions, tests, exported aliases, and imported public
functions do not receive inferred signatures.
When the tail is a local with an explicit refinement annotation, an omitted
private result retains the resolved refinement. This applies to local and
imported ADTs; the inferred result uses the resolved ADT's canonical display
name rather than preserving a module qualifier from the local annotation.
For an omitted private result whose final expression is `if` or `match`, typed
branches with refinements of the same instantiated ADT use the control-flow
result join described below. Equal constructor singletons retain that
singleton. Different singletons can retain a finite refinement set instead of
widening immediately to the base ADT.

Empty `Vec<T>` literals, `Nil` for `List<T>`, and empty dictionary literals
accept concrete expected collection types from local annotations, return
positions, call arguments, record fields, match arm results, constructor
payloads, and compiler-known prelude helper result context for callback return
values. `Nil` in an omitted local binding may also be fixed by a later
same-function use. Empty dictionary literals use `{}` when the expected type
is `Dict<K, V>`; a later same-function use may fix an omitted local `{}`
binding to that dictionary type. Without a dictionary expectation, `{}`
remains an empty record literal. An expected collection type that still
contains `unknown` is not concrete enough for an empty collection literal.
When an empty collection literal still lacks concrete context, its
`type.inference_ambiguous` JSON details identify the empty collection slot,
current inferred type, and empty collection type-context constraint.
Record field and constructor payload expected types propagate recursively
through nested initializer expressions when every enclosing field or payload
type is concrete. This lets empty collection literals and nullary
source-declared constructors inside nested record literals and constructor
payloads use the same concrete context they would receive at the top level.

Payload-carrying ADT constructors infer omitted type arguments from payload
expressions when there is no surrounding expected ADT type. The constructor
name must resolve to one visible variant, and every type argument must become
concrete from the payloads. Repeated uses of the same type parameter join
same-ADT constructor refinements by the aggregate join rule. Other incompatible
later payloads report `type.mismatch` at that payload expression. If payloads
leave a constructor type argument as `unknown`, the
constructor reports `type.inference_ambiguous` with a constructor slot kind,
current inferred type, and constructor type-context constraint. Bare,
type-qualified,
import-alias-qualified, and import-alias-and-type-qualified constructor forms
use the same visibility and descriptor resolution rules as constructor calls
with expected type context. When visible ADTs share an unqualified constructor
leaf, an expected ADT selects the variant owned by that ADT for both nullary
and payload-carrying constructors. Without that expectation the ordinary
ambiguity rule applies. Nullary generic constructors still require surrounding
type context.

Compiler-known collection and option/result helpers propagate concrete callback context.
The input container determines callback item types; an explicit `_with` context is the
first callback argument; a concrete helper result determines the callback return
context. The supported families are:

| Input | Helpers | Callback facts |
| --- | --- | --- |
| `Vec<T>` | `vec_map`, `vec_filter`, `vec_fold`, `vec_try_map`, `vec_try_map_with` | item `T`; `_with` also supplies context |
| `List<T>` | `list_map`, `list_filter`, `list_fold`, `list_try_map` | item `T` |
| `Dict<K,V>` | `dict_map`, `dict_map_with`, `dict_filter`, `dict_filter_with`, `dict_try_map`, `dict_try_map_with`, `dict_fold`, `dict_fold_with` | key `K`, value `V`; folds also constrain accumulator |
| `Option<T>` | `option_map`, `option_and_then` | item `T` |
| `Result<T,E>` | `result_map`, `result_and_then`, `result_map_err` | success `T` or error `E` |

The same expected-function rule applies to ordinary same-module helpers, visible
imported public helpers, public aliases, concrete function-typed local bindings,
record fields, return expressions, `match` arms, `if` branches, constructor
payloads, and collection elements. A named same-module private callback receives
omitted parameter types from one concrete function type and its body return is
checked against the concrete return type. Effects are checked as part of
function compatibility.

A named private callback in an omitted local binding may receive one later
concrete function expected type through one direct binding hop. Aliases of
aliases, imported functions, public boundary signatures, and function types
containing `unknown` do not constrain the callback. Conflicting later uses report
`type.mismatch` at the incompatible use. Source-backed prelude helpers without a
compiler-known callback rule use their concrete embedded source callback
signature; an unknown-containing signature does not constrain the callback.
Public callback signatures, exported aliases, and recursive slots inferred only
from recursion remain uninferred.

Concrete record, constructor, branch, and collection contexts recurse into
initializers. An empty collection or nullary generic constructor requires a
concrete expected type at the context that constrains it; an expected type still
containing `unknown` is insufficient. Empty `Vec`, `Nil`, and dictionary literals
otherwise report `type.inference_ambiguous`; omitted local bindings that remain
unresolved report `type.local_inference_incomplete`.
Record field access gets its result type from the inferred base record type.
Wildcard lets use the same annotation rule as named lets but do not add a
binding to the local environment. Record let patterns bind each nested binding
to the corresponding record field type when the right-hand side or annotation
has a known record type. A record let pattern field missing from a known record
type reports `type.field_missing` at the pattern field.
Constructor let patterns bind each nested binding to the corresponding
constructor payload type when the right-hand side or annotation has a known ADT
descriptor type. A constructor pattern that resolves to a different descriptor
reports `type.mismatch` at the constructor pattern. Pattern bindings whose
payload or field type remains `unknown` still report
`type.local_inference_incomplete` unless another diagnostic already explains
the pattern.

`match` infers a scrutinee type before checking arm bodies. Constructor
patterns can constrain an otherwise unknown scrutinee when the visible arm
patterns identify exactly one finite descriptor domain: `Option<T>`,
`Result<T, E>`, `List<T>`, or one source-declared ADT. Payload literal and
nested constructor subpatterns contribute concrete descriptor type arguments
when they determine them. A catch-all arm alone does not infer the scrutinee
type. Ambiguous constructor-pattern domains leave the scrutinee unknown and
report `type.inference_ambiguous` when a concrete scrutinee type is required.
The JSON details identify the match scrutinee slot, candidate domains, and
constructor-pattern domain constraint.

A binding pattern has the scrutinee type. `Some(value)`,
`Option::Some(value)`, `Ok(value)`, `Result::Ok(value)`, `Err(error)`,
`Result::Err(error)`, `Cons(head, tail)`, and `List::Cons(head, tail)`, and
source-declared constructor patterns bind their payload patterns to the
corresponding descriptor argument when the scrutinee type is known.
Source-declared constructor patterns may use bare, type-qualified,
import-alias-qualified, or import-alias-and-type-qualified names when the
constructor is visible. A qualified constructor pattern whose final segment is
lowercase is rejected by the source identifier casing rule and is not an
accepted constructor case. It is not used for constructor payload typing or
ordinary exhaustiveness coverage. A constructor-pattern type mismatch is still
reported when initial-only repair of the final segment resolves a constructor
for a different ADT descriptor. When direct-bare-binding refined coverage is
not active, the ordinary exhaustiveness path also computes the constructor
found by changing only the invalid final segment's first ASCII lowercase
letter to uppercase and resolving the resulting path through ordinary
case-sensitive lookup. If that constructor is in the matched ADT descriptor,
the ordinary path suppresses a missing-case diagnostic only for that recovered
case and only when the invalid head is the sole cause of the missing case.
This suppression does not make the invalid arm valid. Constructor spellings
that still differ after that initial-only repair remain missing cases. The
invalid pattern's nested binding patterns and arm expression still receive
checking. For `List<A>`,
`head` binds as `A` and `tail` binds as `List<A>`. A record pattern field binds
nested patterns to the corresponding record field type when the scrutinee type
is known. Unknown or non-record scrutinee types leave nested pattern bindings
unknown. Arm expressions share the expected result type when one is available.

When the scrutinee is a bare immutable parameter or local binding, a valid
constructor arm gives that same binding the constructor's singleton refinement
while checking the arm expression. The binding's current type can be the base
ADT, one singleton refinement, or a finite refinement union. The refinement
retains the resolved ADT identity and its instantiated generic arguments.

A singleton or finite-union scrutinee restricts the match domain to its current
variant set. Each valid constructor arm removes its variant from the residual
set. A binding catch-all receives the complete residual refinement, and a
non-binding `_` catch-all gives the existing scrutinee binding that same
refinement. Complete constructor coverage or one catch-all makes the match
exhaustive. A nested `match` observes and can refine the current arm's finite
domain. Leaving an inner arm preserves the enclosing refinement, and leaving
the outer arm restores the binding's original type.

```veln
type Boxed<A>
	Filled(A)
	Empty
end

fn use_filled(value: Boxed<Int>::Filled) -> Int
	1
end

fn inspect(value: Boxed<Int>) -> Int
	match value
		Filled(_) => use_filled(value)
		Empty => 0
	end
end
```

The call in the `Filled` arm is accepted because `value` has type
`Boxed<Int>::Filled` in that arm. After the `match`, `value` again has type
`Boxed<Int>`. The checked
[`adt-variant-refinement-match-binding`](../../examples/specification/check/adt-variant-refinement-match-binding/)
also demonstrates a generic three-variant refined domain whose binding
catch-all, wildcard catch-all, and original scrutinee binding receive the
complete two-variant residual type. Nested matches distinguish that residual
from either singleton narrowing or widening to the base ADT.

For direct bare bindings with a refined domain, arm classification first
validates the constructor name, visibility, owning
ADT, substituted generic payload types, payload arity, nested patterns, and
admitted payload bindings. An invalid arm keeps its intrinsic diagnostic,
consumes no coverage, and produces no derived impossible or redundant
diagnostic. An unresolved constructor path reports `name.unresolved` for the
complete written path at its final segment. A payload pattern incompatible
with the constructor's substituted generic payload type reports the ordinary
`type.mismatch`. A wrong payload arity reports
`type.constructor_pattern_arity` at the complete constructor pattern while
still checking supplied payload patterns and the arm body. Either failure can
leave the match non-exhaustive because the rejected arm covers no variant. A
valid same-ADT constructor outside the original refined domain reports
`type.match_impossible_variant`. A valid constructor or catch-all with an empty
residual reports `type.match_redundant_arm`. An impossible or redundant arm
does not consume coverage.

Impossible and redundant arms still check payload bindings, their body, and an
expected result inherited from the enclosing expression. A redundant
constructor recovers with its original-domain singleton. An impossible
constructor retains the original scrutinee type. A binding catch-all reached
after complete coverage also uses the original scrutinee type because there is
no empty refinement union. The diagnostic specification defines the stable
classification details and related locations.

Every nested constructor must resolve in its expected payload ADT and have the
expected payload arity. Every nested record field must be unique and present in
its expected record type. Every literal and unit payload pattern must match its
expected payload type. Every payload binding introduced by the pattern must be
admitted to the arm scope. Admission rejects invalid value-name casing,
duplicates of another payload binding, parameter, or visible local, and a
`callsite` binding that would shadow the built-in call-site location. A failure
at the arm head, at either kind of nested pattern, at a literal or unit payload,
or while admitting a payload binding leaves the matched binding at its pre-arm
type while the arm expression is checked. The checked
[`adt-variant-refinement-match-binding`](../../examples/specification/check/adt-variant-refinement-match-binding/)
demonstrates parameter, local, generic, nested-arm, and literal-payload use.

This direct refinement requires the scrutinee source to consist only of the
bare binding name. Parenthesized or qualified values, record-field paths, and
transparent aliases do not receive this refinement or refined-domain arm
classification. Calls, constructor expressions, and other computed
scrutinees likewise retain the ordinary base-ADT match behavior.

Without an expected result, refinements of the same ADT identity join by taking
the union of their variant sets when every generic argument is fully resolved
and identical. The union is independent of arm order and renders in ADT
declaration order. A refinement joined with its base ADT, or a union containing
every declared variant, produces the base ADT. Once the first refinement starts
such a join, later arm results are resolved independently before they contribute
to it. The join does not resolve an ambiguous constructor, infer a missing or
nested unknown generic argument, or supply an expected type to a sibling arm.
An explicit enclosing expected type continues to flow to every arm. A later
result that cannot join reports the ordinary compatibility diagnostic against
the base ADT. That failure abandons the finite join: recovery uses the base ADT,
and a later compatible refinement does not resume the partial join.
When the first typed arm cannot start an ADT-refinement join, it supplies the
initial result type for the existing compatibility and mismatch rules. When
that type is a concrete base ADT, it supplies context to later arms, including
an otherwise ambiguous generic constructor. The result remains the base ADT.

`if` and `else if` conditions are checked with expected type `Bool`. A
non-`Bool` condition reports `type.mismatch` at the condition expression.
Branch body expressions share the expected result type when one is available.
Without one, branch results use the same symmetric ADT-refinement join as
`match`; all other combinations retain the compatibility behavior described
above. Typed holes in conditions therefore receive `Bool`. A hole in a branch
receives the enclosing expected result type when one exists. A result join does
not itself provide that expectation. The checked control-flow cases are in
[`adt-variant-refinement-control-flow-result-joins`](../../examples/specification/check/adt-variant-refinement-control-flow-result-joins/)
and its
[`diagnostic companion`](../../examples/specification/check/adt-variant-refinement-control-flow-result-joins-diagnostics/).

After scrutinee type inference and arm expression checking, `match` expressions
over finite domains must be exhaustive. `Bool` scrutinees require coverage for
`true` and `false`; `Option<T>` scrutinees require `Some(_)` and `None`;
`Result<T, E>` scrutinees require `Ok(_)` and `Err(_)`; `List<A>` scrutinees
require `Nil` and `Cons(_)`; source-declared ADT scrutinees require every
declared variant. Variant visibility does not change that finite domain. In an
importing module, private source-declared constructors still require coverage,
so arms for every public constructor are not exhaustive by themselves. Use `_`
or a binding catch-all arm because the private constructors cannot be named
there. `_` and binding patterns are catch-all arms. A
direct bare binding whose current type is a singleton or finite refinement
union instead requires only that restricted original domain. A valid
out-of-domain constructor is impossible and does not satisfy or expand the
domain. A non-exhaustive finite-domain match reports
`type.match_non_exhaustive` at the `match` expression. The missing case is the
unqualified coverage label: source-declared ADTs use the constructor leaf name,
with `_` for payload variants. Related notes identify the scrutinee type and
the arms that prove partial coverage.

Coverage classification for a direct refined domain has linear analysis work
when the domain size and the number of arms grow together. This bound covers
complete and incomplete coverage, duplicate and impossible constructor arms,
and a catch-all after complete constructor coverage. For a fixed singleton
domain, widening the base ADT does not increase coverage setup work or retained
match-local slots. Increasing the number of sequential singleton matches
increases setup work linearly while retaining one match state at a time.
Increasing singleton-match nesting depth increases peak retained slots
linearly, independently of the base ADT width.

Deterministic test counters measure coverage work, initialized collection
slots, peak retained collection slots, and copied diagnostic labels across
adjacent generated input sizes. The collection counts include domain ranks,
lookup slots, covered-arm order, and a catch-all arm's temporary residual
ranks. Allocation growth and release contribute symmetrically, and retained
slots return to zero after analysis. These counters, rather than elapsed time,
define the regression checks; reported wall-clock timings are observational.
Final serialized JSON can still grow quadratically when a linear number of
diagnostics must each expose the complete refined domain.

### Result propagation

Postfix `?` unwraps the success type `T` from an operand whose type is
`Result<T, E>`. It uses the existing propagation context rules, including the
prohibition inside a deferred block described above. When the enclosing
function or test returns `Result<_, F>`, `E` must satisfy the existing
propagation error-compatibility rule for `F`. An incompatible error type
reports the ordinary `type.mismatch` diagnostic at the operand.

The operator accepts the base `Result<T, E>`, either singleton refinement
`Result<T, E>::Ok` or `Result<T, E>::Err`, and the complete refinement union.
Every form uses the same context and error-compatibility checks. The expression
type is exactly `T`, including when `T` is itself a variant refinement.

At runtime, a success value produces its payload and evaluation continues. An
error value returns through the enclosing function's existing propagation
path. A singleton `Ok` or `Err` type determines that branch statically but does
not change the checks or runtime representation. Source following a statically
known `Err` is still checked normally.

Checked source and diagnostic evidence is in
[`adt-variant-refined-result-propagation`](../../examples/specification/check/adt-variant-refined-result-propagation/),
its
[`human`](../../examples/specification/check/adt-variant-refined-result-propagation-diagnostics-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refined-result-propagation-diagnostics-json/)
diagnostic cases, and the
[`JVM execution case`](../../examples/specification/run/adt-variant-refined-result-propagation/).

## Compatibility and limits

The type checker resolves a structurally valid `A<T>::V` annotation to the
singleton variant type for `V` of the finite ADT `A<T>`. A union of alternatives
for the same ADT identity and generic arguments denotes their finite variant
set. The alternatives are resolved before their base identities are compared,
so qualified and unqualified spellings of the same ADT can form one union.
Duplicate alternatives are removed and display follows ADT declaration order.
`Option<T>`, `Result<T, E>`, `List<T>`, and source-defined ADTs use this same
representation. A type alias cannot qualify a variant refinement in this
slice; an alias-qualified singleton or union alternative is an invalid type
annotation. A union containing every declared variant is equivalent to the base
ADT. An unknown variant, invalid base arity, or union of different resolved ADTs
is an invalid type annotation; it does not become an assignable `unknown`
contract.

A resolved constructor expression has its singleton variant type. The expected
base ADT can supply generic arguments to the constructor, and the singleton can
widen directly to that base without a runtime conversion. An unannotated local
binding retains the singleton, including while a later expected type fills an
unknown generic argument. The later constraint does not replace the
constructor's variant identity. Core lowering erases the refinement to the
base ADT, so constructor tags, payloads, and runtime representation are
unchanged.

An unannotated record literal retains the inferred type of each field
initializer. A constructor initializer therefore gives its field the
constructor singleton type. Field access exposes that retained type, including
when the field is passed to a parameter that requires the same singleton. An
omitted private helper result inferred from such a record retains the same
field type.

An expected record type supplies the expected type of each matching field
during construction. A constructor singleton can widen at that direct field
boundary, so `{state: Ready}` can construct an expected `{state: State}`.
Without that expected record type, the inferred value is
`{state: State::Ready}`. A later assignment of that value to
`{state: State}` is rejected because direct refinement widening does not
recurse through records.

Unannotated collection elements, dictionary keys and values, and inferred
generic ADT payload positions retain constructor refinements. Several values
contributing to one position use the same declaration-ordered join rule as
collection inference. Constructor payload patterns and collection helpers
observe the retained type argument after substitution. Private omitted-result
inference applies these rules too, so its inferred signature agrees with
ordinary body inference. If an element, entry, or payload expression fails to
type-check, its recovered type does not contribute to the aggregate join. The
failed expression leaves the previously accumulated join unchanged for later
successful contributions and for an inferred private result. This exclusion
also applies when a call argument or constructor payload fails but recovery can
still identify the call or constructor's refined result type.

The join applies when repeated constructor payloads use a type parameter
directly. If the same parameter also occurs inside an invariant named payload,
that nested occurrence instead establishes an exact constraint. Every direct
or nested contribution must then match that constraint; the checker does not
widen across the nested named type. For example, given `Built(A, Vec<A>)`,
`Built(Ready, [Ready])` infers `Container<State::Ready>::Built`, while
`Built(Ready, [Closed])` reports `type.mismatch`. Reversing the two payload
positions does not change either result.

All occurrences contributed by one payload are accepted or rejected together.
If one occurrence conflicts, no occurrence from that payload constrains the
constructor type argument. A later valid payload therefore continues from the
last successfully inferred type instead of from a partial result of the failed
payload. This rule also applies when the rejected payload contains repeated
occurrences inside one invariant named type.

When a concrete payload has the form `Box<A>`, a matching refined carrier such
as `Box<State::Ready>::Boxed` can supply `State::Ready` for `A`. This direct
carrier inference allows `Carried(Boxed(Ready))` to infer
`Carrier<State::Ready>::Carried`. It does not make named type arguments
covariant: a later use of `Box<State::Ready>` where `Box<State>` is required
still reports `type.mismatch`.

An explicit aggregate component type supplies context while the aggregate is
constructed. A constructor singleton can widen directly at that component
boundary, so `[Ready]` can construct an explicitly expected `Vec<State>` and
`{State::Ready: 1}` can construct an explicitly expected `Dict<State, Int>`.
The same rule lets `Boxed(Ready)` construct an explicitly expected
`Box<State>`. Without that context, the values retain `Vec<State::Ready>`,
`Dict<State::Ready, Int>`, and `Box<State::Ready>::Boxed`. A later assignment
from any inferred value to the corresponding base-argument aggregate is
rejected. Named type arguments remain invariant; aggregate inference does not
add nested covariance.

At a direct assignment, argument, or result boundary, variant assignability is
defined as follows:

| Actual | Expected | Outcome |
| --- | --- | --- |
| `A<T>::V` | `A<T>::V` | Accepted. |
| `A<T>::V` | `A<T>::V \| A<T>::W` | Accepted. |
| `A<T>::V \| A<T>::W` | a same-base superset | Accepted. |
| a variant set | a same-base strict subset | Rejected. |
| a singleton or variant set | `A<T>` | Accepted by direct widening. |
| `A<T>` | a singleton or variant set | Rejected. |
| `A<T>::W` | a different singleton `A<T>::V` | Rejected. |

The base ADT identity is nominal and includes the resolved owning module. A
refinement of one ADT cannot widen to a same-spelled base ADT from another
module.

The type checker applies the same rules to refined call parameters and declared
function results. A final `if` or `match` checks every successfully typed branch
or arm against the declared result, including a branch excluded by a constant
condition. An earlier error that leaves an expression untyped does not add a
derivative refinement mismatch.

Refinement widening is direct only. It does not recurse through named type
arguments, records, ADT payload types, or function parameter, variadic, or
result positions. A refinement-bearing nested position must be identical on
both sides. Function values therefore remain compatible only when every
refinement-bearing position matches, in addition to the existing function
shape and effect rules.

An incompatible complete refinement comparison reports
`type.variant_mismatch` at the assigned expression, call argument, branch, arm,
or final result. Its JSON details contain the rendered `actual_type`, rendered
`expected_type`, declaration-ordered `expected_variants`, and an
`excluded_variants` fact. That fact has `form: listed` and a declaration-ordered
`variants` array when the actual type is a finite refinement. It has
`form: all_except_expected` and an empty `variants` array when the actual type
is the complete base ADT. A `variant_exclusion` related note renders the same
fact for human output, and another related note identifies the expected local
annotation, parameter, result declaration, or compiler-known helper parameter
inferred at the call site. A nested record, named argument, ADT payload, or
function-position invariance failure has no truthful top-level variant
exclusion and uses the ordinary `type.mismatch` diagnostic instead.
The checked examples cover
accepted source and compiler-known cases in
`examples/specification/check/adt-variant-refinement-call-typing/`, JSON failures
in `examples/specification/check/adt-variant-refinement-call-typing-diagnostics-json/`,
and human diagnostics in
`examples/specification/check/adt-variant-refinement-call-typing-diagnostics-human/`.
Aggregate retention, joins, contextual widening, projection, and omitted
private results are checked in
`examples/specification/check/adt-variant-refinement-aggregate-retention/`;
rejected nested widening and generic argument mismatch are checked in its
`-diagnostics` companion.

Alias spelling and provenance, public/private exposure paths, pattern-based
control-flow refinement beyond direct bare immutable bindings, schema
boundaries, package-documentation signatures, command-wide coverage, LSP, MCP,
and language-reference publication remain proposal work. This slice also does
not add recursive generic or function variance.

Assignment compatibility treats `unknown` as compatible with any type. Record
assignment is width-compatible: every expected field must exist in the actual
record and be assignable. Named types with the same constructor are compatible
when their arguments are pairwise compatible at a nested boundary. A nested
boundary accepts `unknown`, so `Vec<unknown>` accepts `Vec<Int>`, but it does
not apply direct refinement widening: `Vec<State::Ready>` does not satisfy
`Vec<State>`. `Path` and `String` are distinct named types at assignment
boundaries; the runtime path representation is not source-visible.
Function assignment checks fixed parameter count, parameter types, variadic
shape, return type, and effects. Variadic and fixed-arity function types are
not assignment-compatible with each other. Two variadic function types are
compatible only when the fixed parameters and variadic element types are
assignable. The actual callable's effects must all be present in the expected
function type's effect list, so a pure callable can satisfy an effectful
function type but a `stdio` callable cannot satisfy a pure function type. If
the expected function type contains a bound final effect row tail such as
`effects [stdio, ...E]`, the row tail accepts the actual callable effects not
already named by the concrete entries. The call boundary substitutes those
effects for `E` when it computes the enclosing call's concrete effect set.

One record literal cannot declare the same field name twice. Duplicate record
literal fields are name errors before record assignability chooses an expected
field type.

Dictionary literals infer `Dict<K, V>` from their expected type when available.
An empty `{}` expression becomes an empty dictionary only when the expected type
is `Dict<K, V>`. Without an expected dictionary type, the first entry supplies
the initial key and value contributions. Each later entry contributes to the
same positions. Compatible same-ADT refinements join as described above;
otherwise, an incompatible key or value reports `type.mismatch`. A dictionary
key may be any implemented expression; the parser only reserves a first bare
`name: value` entry for record literals.

Record field access `expr.name` requires the base expression to have a record
type containing `name`. The access has the declared field type. Accessing a
field absent from a known record type is a type error reported at the field
name, with the base expression reported as related context.

## Operators and diagnostics

Implemented operator typing:

- `not` expects `Bool` and returns `Bool`.
- Unary `-` expects `Int` and returns `Int`, or expects `Float` and returns
  `Float` when the expected result type or operand is clearly `Float`.
- Unary `~` expects `Int` and complements all 64 bits.
- `or` and `and` expect `Bool` operands and return `Bool`.
- `&`, `^`, and `|` expect `Int` operands and return their bitwise AND, XOR,
  and OR result as `Int`.
- `<<`, `>>`, and `>>>` expect `Int` operands. `<<` discards shifted-out high
  bits, `>>` extends the sign bit, and `>>>` fills high bits with zero. A
  literal count outside `0..63` reports `type.invalid_shift_count` at the
  count expression. A dynamic invalid count fails with
  `runtime.invalid_shift_count`; shift counts are never masked modulo 64.
- comparisons other than equality expect matching `Int` operands or matching
  `Float` operands and return `Bool`. A `Float` expected result does not apply
  to comparisons, so `Float` comparison is selected from the operand types.
- `+`, `-`, `*`, and `/` expect `Int` operands and return `Int`, or expect
  numeric operands and return `Float` when the expected result type or either
  operand is clearly `Float`.
- `==` and `!=` return `Bool` and do not currently require matching operand
  types.
- `|>` requires a named or qualified call expression on the right. The left
  expression is checked as the first argument of that call, and the pipeline
  result is the call result. A non-call target, or a call whose callee is not a
  name path, reports `type.pipeline_target`.

Operator typing permits `Int` operands where a selected `Float` operator
expects a numeric operand. This widening is limited to numeric operators;
ordinary assignment, return, record, vec, and call argument checking still
require `Float` where `Float` is declared.

Float arithmetic and comparison operators lower as calls to compiler-known
prelude functions. `Float` values follow the backend floating-point value
space, including infinities and NaN values.

Integer bitwise operations use signed 64-bit two's-complement patterns on
every backend. Their precedence from highest to lowest is prefix, multiply,
add, shift, comparison, equality, `&`, `^`, `|`, `and`, `or`, and pipeline.
The formatter writes spaces around binary operators and no space after unary
`~`. Contract typing and runtime checks accept these operators, and static
contract reasoning evaluates literal-only bitwise expressions. Repair
reasoning leaves nonliteral bitwise predicates runtime-checked instead of
inventing an arithmetic rewrite.

## References

- Type inference and compatibility: `crates/veln-sema/src/types.rs` and
  `crates/veln-sema/src/type_annotation_parser.rs`.
- Effect-row typing: `crates/veln-sema/src/effect_rows.rs`.
- Refined-match coverage scaling:
  `crates/veln-sema/src/tests/variant_refinement_match_scaling.rs`.
- Parser coverage: `crates/veln-syntax/src/tests/calls_and_generics.rs`,
  `literals_and_numbers.rs`, and `patterns_and_control_flow.rs`.
