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
- `match` arm results and constructor payload bindings
- record pattern field bindings in `match` arms and `let` statements

When a local `let` binding omits its annotation and its initializer leaves the
binding type with `unknown`, later same-function uses may fix the binding to
one concrete type. Implemented constraining uses include call arguments and
tail expressions checked against a declared return type. The binding remains
monomorphic: after one concrete type is fixed, a later incompatible use reports
`type.mismatch`. If no same-function use fixes every `unknown` part of the
binding type, checking reports `type.local_inference_incomplete` at the
omitted binding.

Record field access gets its result type from the inferred base record type.
Wildcard lets use the same annotation rule as named lets but do not add a
binding to the local environment. Record let patterns bind each nested binding
to the corresponding record field type when the right-hand side or annotation
has a known record type.

`match` infers the scrutinee first. A binding pattern has the scrutinee type.
`Some(value)`, `Option::Some(value)`, `Ok(value)`, `Result::Ok(value)`,
`Err(error)`, `Result::Err(error)`, `Cons(head, tail)`, and
`List::Cons(head, tail)`, and source-declared constructor patterns bind their
payload patterns to the corresponding descriptor argument when the scrutinee
type is known. Source-declared constructor patterns may use bare,
type-qualified, import-alias-qualified, or import-alias-and-type-qualified
names when the constructor is visible. For `List<A>`, `head` binds as `A` and
`tail` binds as `List<A>`. A record pattern field binds nested patterns to the
corresponding record field type when the scrutinee type is known. Unknown or
non-record scrutinee types leave nested pattern bindings unknown. Arm
expressions share the expected result type when one is available; otherwise the
first arm supplies the initial result type for later arms.

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
Without an expected dictionary type, the first entry supplies the initial key
and value types. Later entries are checked against the same key and value
expectations. A dictionary key may be any implemented expression; the parser
only reserves a first bare `name: value` entry for record literals.

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
