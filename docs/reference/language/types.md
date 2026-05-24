# Types

This file specifies implemented type annotations, inference, assignment
compatibility, and operator typing.

## Annotations

Implemented type annotations:

- primitives: `Bool`, `Int`, `Float`, `String`, and `()`
- built-in type constructors: `Option(T)`, `Result(T, E)`, `List(T)`, and
  `Dict(K, V)`
- records: `{name: Type, ...}`
- function types: `fn(T, ...) -> U` with optional `effects [name, ...]`
- other named type paths with optional type arguments, unless they are one of
  the arity-checked built-ins above

One record type annotation cannot declare the same field name twice. A
duplicate field in a record type annotation is an invalid type annotation.

Public functions must annotate every parameter, annotate the return type, and
provide an explicit `effects [...]` clause. Private functions may omit these
annotations.

The optional result binding in `-> name: Type` names the return value for
postconditions, but the type annotation remains `Type`.

Test declarations must use an empty parameter list, annotate the return type as
`()` or `Result((), E)`, and provide an explicit `effects [...]` clause. Their
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
- list elements
- dictionary keys and values
- `Ok`, `Err`, `Some`, `None`, and postfix `?`
- `match` arm results and constructor payload bindings
- record pattern field bindings

Record field access gets its result type from the inferred base record type.

`match` infers the scrutinee first. A binding pattern has the scrutinee type.
`Some(value)`, `Ok(value)`, and `Err(error)` bind their payload patterns to
the corresponding `Option` or `Result` argument when the scrutinee type is
known. A record pattern field binds nested patterns to the corresponding record
field type when the scrutinee type is known. Unknown or non-record scrutinee
types leave nested pattern bindings unknown. Arm expressions share the expected
result type when one is available; otherwise the first arm supplies the initial
result type for later arms.

## Assignment Compatibility

Assignment compatibility treats `unknown` as compatible with any type. Record
assignment is width-compatible: every expected field must exist in the actual
record and be assignable. Named types with the same constructor are compatible
when their arguments are pairwise assignable, so `List(unknown)` accepts
`List(Int)`. Function assignment checks parameter count, parameter types, and
return type; effect lists are currently carried but not compared for
function-type assignability.

One record literal cannot declare the same field name twice. Duplicate record
literal fields are name errors before record assignability chooses an expected
field type.

Dictionary literals infer `Dict(K, V)` from their expected type when available.
Without an expected dictionary type, the first entry supplies the initial key
and value types. Later entries are checked against the same key and value
expectations.

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
- `|>` requires a call expression on the right. The left expression is checked
  as the first argument of that call, and the pipeline result is the call
  result. A non-call target reports `type.pipeline_target`.

Operator typing permits `Int` operands where a selected `Float` operator
expects a numeric operand. This widening is limited to numeric operators;
ordinary assignment, return, record, list, and call argument checking still
require `Float` where `Float` is declared.

Float arithmetic and comparison operators lower as calls to compiler-known
prelude functions. `Float` values follow the backend floating-point value
space, including infinities and NaN values.
