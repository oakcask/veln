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

Public functions must annotate every parameter, annotate the return type, and
provide an explicit `effects [...]` clause. Private functions may omit these
annotations.

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
- record fields
- list elements
- `Ok`, `Err`, `Some`, `None`, and postfix `?`

## Assignment Compatibility

Assignment compatibility treats `unknown` as compatible with any type. Record
assignment is width-compatible: every expected field must exist in the actual
record and be assignable. Function assignment checks parameter count, parameter
types, and return type; effect lists are currently carried but not compared for
function-type assignability.

## Operators

Implemented operator typing:

- `not` expects `Bool` and returns `Bool`.
- Unary `-` expects `Int` and returns `Int`.
- `or` and `and` expect `Bool` operands and return `Bool`.
- comparisons other than equality expect `Int` operands and return `Bool`.
- `+`, `-`, `*`, and `/` expect `Int` operands and return `Int`.
- `==` and `!=` return `Bool` and do not currently require matching operand
  types.
- `|>` is parsed and lowered as a binary operator with unknown operand and
  result types. It has no special call-rewrite semantics in the implemented
  slice. The current JVM runtime helper returns the right operand.
