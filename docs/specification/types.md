# Types

This page routes implemented type-system facts. Use it before opening the
full type reference.

## Read First

- Type annotations include primitives, descriptor-backed `Option`, `Result`,
  `List`, source-declared ADTs, built-in containers, records, function types,
  named type paths, and optional result bindings. Function type parameter
  lists may use a final variadic `...T` element. Type constructor arguments
  use angle brackets; parenthesized type constructor arguments are rejected in
  source type positions.
- Local inference is monomorphic and flow-sensitive within one function body.
  An omitted local `let` binding type may be fixed by a later same-function use
  such as a call argument or return-compatible expression when that use
  requires one concrete type. Non-empty `Vec<T>` and `Dict<K, V>` literal
  initializers may also infer omitted local binding types when every element,
  key, and value agrees on one concrete type. Empty `Vec<T>` literals, `Nil`
  for `List<T>`, and empty dictionary literals accept concrete expected
  collection types from annotations, returns, call arguments, record fields,
  match arms, `if` branches, and constructor payloads, plus compiler-known
  prelude helper result context for callback return values. Concrete record
  field and constructor payload expected types also propagate through nested
  initializer expressions when every enclosing field or payload type is
  concrete. Payload-carrying
  ADT constructors also infer omitted type arguments from payload expressions
  when the constructor resolves to one visible variant and every type argument
  becomes concrete.
  Record let patterns bind nested named fields from a known record initializer
  or local annotation; missing fields report `type.field_missing`.
  Constructor let patterns bind named payload positions from a known ADT
  initializer or local annotation; wrong descriptor constructors report
  `type.mismatch`.
  Constructor patterns in `match` arms may constrain an otherwise unknown
  scrutinee when the visible arms identify one finite descriptor domain.
  Compiler-known collection, dictionary, option, and result helper input
  types also constrain named private callback function parameters passed to
  the implemented map, filter, fold, try-map, `vec_try_map_with`,
  context-carrying dictionary aliases, and and-then helpers. Same-module
  helpers and visible imported
  helpers whose declared parameter type is a concrete function type also
  constrain named private callback parameters at that argument position,
  including the fixed parameter types and variadic element type of a concrete
  variadic function type.
  Source-backed prelude helpers without a compiler-known callback rule use the
  same declared-helper fallback when their embedded source signature contains a
  concrete function-typed callback parameter. A concrete expected record field
  whose type is a concrete function type also constrains a named private
  callback placed in that record field initializer.
  A local binding whose annotation is a concrete function type also constrains
  a named private callback assigned as that binding initializer; later calls or
  returns through that local binding use the same concrete function type.
  A direct function body return position whose declared return type is a
  concrete function type also constrains a named private callback returned from
  that body.
  A constructor payload whose expected type is a concrete function type also
  constrains a named private callback placed in that payload position. This
  includes compiler-owned `Some`, `Ok`, and `Err` payloads. When such a
  concrete helper, record-field, local-binding, direct return, constructor
  payload, or prelude helper result context fixes a named private callback
  return type, that expected return type propagates into non-empty callback
  tail expressions such as `Some(...)`, `Ok(...)`, `Err(...)`, source ADT
  constructors, records, and collection literals.
- Private non-exported helper functions may omit parameter and return
  annotations when same-module concrete call sites and body facts determine one
  monomorphic signature. Public functions, tests, exported aliases, and
  imported public functions still require declared signature boundaries.
- `Option<T>` and `Result<T, E>` are compiler-owned built-in ADTs. `List<T>`
  and source-declared ADTs are descriptor-backed. Their constructors, payload
  bindings, result propagation where applicable, and finite-domain
  exhaustiveness use descriptor facts.
- The standard prelude exposes source-visible `Byte`, `ByteChunk`,
  `ByteView`, `ByteOffset`, `ByteCount`, `StreamInput`,
  `AcceptOutcome`, `StreamReadOutcome`, `StreamWriteOutcome`,
  `DecodeStep<T>`, `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`,
  and `EncodeError` named types for small immutable byte values, bounded byte
  views, byte-counted helper APIs, outgoing chunk lists, listener accept
  decisions, stream read and write decisions, and incremental codec boundary
  values.
- `match` expressions over `Bool`, `Option<T>`, `Result<T, E>`, `List<T>`, and
  source-declared ADTs must be exhaustive unless a catch-all arm is present.
  `if` expressions require a final `else`; `if` and `else if` conditions
  follow the same Boolean branching type rules as equivalent `match Bool`
  expressions. Non-`Bool` conditions and incompatible branch result types
  report `type.mismatch` at the failed condition or branch expression.
- Assignment compatibility treats `unknown` as compatible with any type and
  checks records by required fields. Function compatibility preserves
  fixed-arity versus variadic shape. `Path` is distinct from `String`; the
  runtime path representation is not source-visible.
- Operators use the implemented `Bool`, `Int`, and `Float` rules.

## Read When

- Annotation syntax, public/private annotation requirements, and test
  declaration type requirements: [types-full.md](types-full.md#annotations).
- Local inference sources, match patterns, and pattern let bindings:
  [types-full.md](types-full.md#inference).
- Record, dictionary, function, and field-access assignment compatibility:
  [types-full.md](types-full.md#assignment-compatibility).
- Unary, boolean, comparison, arithmetic, equality, pipeline, and float rules:
  [types-full.md](types-full.md#operators).

## Skip Unless Needed

- Use [source-surface.md](source-surface.md) first when the question is about
  source grammar rather than type behavior.
- Use [contracts-holes.md](contracts-holes.md) for contract and hole typing
  routes before opening full type details.
