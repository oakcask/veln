# Types

This file specifies implemented type annotations, inference, assignment
compatibility, and operator typing.

## Annotations

Implemented type annotations:

- primitives: `Bool`, `Int`, `Float`, `String`, and `()`
- built-in and descriptor-backed type constructors: `Option<T>`,
  `Result<T, E>`, `List<T>`, `Vec<T>`, and `Dict<K, V>`
- standard prelude byte and codec vocabulary names: `Byte`, `ByteChunk`,
  `ByteView`, `ByteOffset`, `ByteCount`, `StreamInput`,
  `AcceptOutcome`, `StreamReadOutcome`, `DecodeStep<T>`,
  `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`, and `EncodeError`
- records: `{name: Type, ...}`
- function types: `fn(T) -> U`, `fn(T, U) -> V`, or `fn(T, ...U) -> V`
  with optional `effects [name, ...]`
- other named type paths with optional type arguments, unless they are one of
  the arity-checked built-ins above

Angle brackets are the source spelling for type constructor arguments. Legacy
parenthesized type constructor arguments in type positions are invalid type
annotations.

`Option<T>` and `Result<T, E>` are compiler-owned built-in ADTs. `List<T>` and
source-declared ADTs use descriptor entries for constructor payload typing,
qualified and unqualified constructor names, postfix `?` result propagation for
`Result`, and finite-domain exhaustiveness. Source ADTs may be generic and
recursive through variant payloads. Constructor payload types instantiate the
declared type parameters from surrounding context and payload expressions.
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
[names-effects-full.md#helper-signatures](names-effects-full.md#helper-signatures).

In a function or test return annotation, a returned function type may carry its
own effect list before the enclosing declaration's effect list. For example,
`-> fn(String) -> () effects [stdio] effects []` returns a callback that may
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

Public functions must annotate every parameter, annotate the return type, and
provide an explicit `effects [...]` clause. Private functions may omit a
parameter or return annotation only when local inference produces a concrete
type for the omitted fact. If the checker still has `unknown`, it reports
`type.private_inference_incomplete`.

The optional result binding in `-> name: Type` names the return value for
postconditions, but the type annotation remains `Type`.

Test declarations must use an empty parameter list, annotate the return type as
`()` or `Result<(), E>`, and provide an explicit `effects [...]` clause. Their
declared effect list is checked against directly inferred effects, but test
declarations are not callable function values.

## Inference

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

Typed holes use the same concrete expected-type flow as other subexpressions.
When a hole appears under a concrete return, call argument, record field, `if`
branch, `match` arm, or constructor payload context, the hole diagnostic and
JSON details report that type and use it to build advisory symbol candidate
queries.

When a local `let` binding omits its annotation and its initializer leaves the
binding type with `unknown`, later same-function uses may fix the binding to
one concrete type. Implemented constraining uses include call arguments and
tail expressions checked against a declared return type. The binding remains
monomorphic: after one concrete type is fixed, a later incompatible use reports
`type.mismatch`. If no same-function use fixes every `unknown` part of the
binding type, checking reports `type.local_inference_incomplete` at the
omitted binding.

Non-empty `Vec<T>` literal initializers infer an omitted local binding as
`Vec<T>` from the first element when all later elements are assignable to the
same concrete element type. Non-empty `Dict<K, V>` literal initializers infer
`Dict<K, V>` from the first key and value when later keys and values are
assignable to the same concrete key and value types. Conflicting later facts
remain focused `type.mismatch` diagnostics at the incompatible element, key,
or value rather than widening the binding type.

When a private non-exported helper omits parameter or return annotations,
same-module concrete call sites may constrain the helper's single monomorphic
signature. Concrete argument expressions constrain omitted parameters. A
concrete expected result type at a helper call constrains an omitted return
type, and body tail facts are checked against the inferred return type. Body
facts and call-site facts must agree; a later incompatible call reports
`type.mismatch` at the failed argument or expected-result use. Direct recursive
edges do not supply inference facts for the recursive helper itself, so an
omitted recursive slot still needs a non-recursive concrete fact or an
annotation. Public functions, tests, exported aliases, and imported public
functions do not receive inferred signatures.

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

Payload-carrying ADT constructors infer omitted type arguments from payload
expressions when there is no surrounding expected ADT type. The constructor
name must resolve to one visible variant, and every type argument must become
concrete from the payloads. Repeated uses of the same type parameter must agree;
an incompatible later payload reports `type.mismatch` at that payload
expression. If payloads leave a constructor type argument as `unknown`, the
constructor reports `type.inference_ambiguous`. Bare, type-qualified,
import-alias-qualified, and import-alias-and-type-qualified constructor forms
use the same visibility and descriptor resolution rules as constructor calls
with expected type context. Nullary generic constructors still require
surrounding type context.

Compiler-known prelude helpers push concrete input item types into named
private callback function values. For `vec_map`, `vec_filter`, `vec_fold`, and
`vec_try_map`, a concrete `Vec<T>` input constrains the callback parameter that
receives each element to `T`. The same rule applies to `list_map`,
`list_filter`, `list_fold`, and `list_try_map` for concrete `List<T>` inputs.
For concrete `Dict<K, V>` inputs, `dict_map`, `dict_map_with`,
`dict_filter`, `dict_filter_with`, `dict_try_map`, and `dict_try_map_with`
constrain callback parameters that receive each key and value to `K` and `V`.
`dict_fold` and `dict_fold_with` constrain accumulator, key, and value
parameters from the fold result context and dictionary input. The `_with`
aliases accept an explicit context argument before the dictionary and pass it
as the first callback argument.
`option_map` and `option_and_then` constrain their callback parameter from the
`Option<T>` input. `result_map` and `result_and_then` constrain their callback
parameter from the `Result<T, E>` success type, and `result_map_err` constrains
its callback parameter from the error type. These helpers still use the
surrounding expected result type to constrain the callback return type when
that expected result is concrete. This inference does not apply to ordinary
user-defined higher-order helpers.

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

A binding pattern has the scrutinee type. `Some(value)`,
`Option::Some(value)`, `Ok(value)`, `Result::Ok(value)`, `Err(error)`,
`Result::Err(error)`, `Cons(head, tail)`, and `List::Cons(head, tail)`, and
source-declared constructor patterns bind their payload patterns to the
corresponding descriptor argument when the scrutinee type is known.
Source-declared constructor patterns may use bare, type-qualified,
import-alias-qualified, or import-alias-and-type-qualified names when the
constructor is visible. For `List<A>`, `head` binds as `A` and `tail` binds as
`List<A>`. A record pattern field binds nested patterns to the corresponding
record field type when the scrutinee type is known. Unknown or non-record
scrutinee types leave nested pattern bindings unknown. Arm expressions share
the expected result type when one is available; otherwise the first arm
supplies the initial result type for later arms.

`if` and `else if` conditions are checked with expected type `Bool`. A
non-`Bool` condition reports `type.mismatch` at the condition expression.
Branch body expressions share the expected result type when one is available;
otherwise the first branch supplies the initial result type for later branches,
matching the result-unification behavior of equivalent `match Bool` arms.
Typed holes in conditions therefore receive `Bool`, while typed holes in
branch bodies receive the enclosing expected result type when one exists.

After scrutinee type inference and arm expression checking, `match` expressions
over finite domains must be exhaustive. `Bool` scrutinees require coverage for
`true` and `false`; `Option<T>` scrutinees require `Some(_)` and `None`;
`Result<T, E>` scrutinees require `Ok(_)` and `Err(_)`; `List<A>` scrutinees
require `Nil` and `Cons(_)`; source-declared ADT scrutinees require every
declared variant. In an importing module, hidden source-declared constructors
still belong to the finite domain, so public constructor arms alone are not
exhaustive; use `_` or a binding catch-all arm when private constructors may be
present. `_` and binding patterns are catch-all arms. A
non-exhaustive finite-domain match reports
`type.match_non_exhaustive` at the `match` expression. The missing case is the
unqualified coverage label: source-declared ADTs use the constructor leaf name,
with `_` for payload variants. Related notes identify the scrutinee type and
the arms that prove partial coverage.

## Assignment Compatibility

Assignment compatibility treats `unknown` as compatible with any type. Record
assignment is width-compatible: every expected field must exist in the actual
record and be assignable. Named types with the same constructor are compatible
when their arguments are pairwise assignable, so `Vec<unknown>` accepts
`Vec<Int>`. `Path` and `String` are distinct named types at assignment
boundaries; the runtime path representation is not source-visible.
Function assignment checks fixed parameter count, parameter types, variadic
shape, return type, and effects. Variadic and fixed-arity function types are
not assignment-compatible with each other. Two variadic function types are
compatible only when the fixed parameters and variadic element types are
assignable. The actual callable's effects must all be present in the expected
function type's effect list, so a pure callable can satisfy an effectful
function type but a `stdio` callable cannot satisfy a pure function type.

One record literal cannot declare the same field name twice. Duplicate record
literal fields are name errors before record assignability chooses an expected
field type.

Dictionary literals infer `Dict<K, V>` from their expected type when available.
An empty `{}` expression becomes an empty dictionary only when the expected type
is `Dict<K, V>`. Without an expected dictionary type, the first entry supplies
the initial key and value types. Later entries are checked against the same key
and value expectations. A dictionary key may be any implemented expression; the
parser only reserves a first bare `name: value` entry for record literals.

Record field access `expr.name` requires the base expression to have a record
type containing `name`. The access has the declared field type. Accessing a
field absent from a known record type is a type error reported at the field
name, with the base expression reported as related context.

## Operators

Implemented operator typing:

- `not` expects `Bool` and returns `Bool`.
- Unary `-` expects `Int` and returns `Int`, or expects `Float` and returns
  `Float` when the expected result type or operand is clearly `Float`.
- `or` and `and` expect `Bool` operands and return `Bool`.
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
