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
  requires one concrete type. Empty `Vec<T>` literals, `Nil` for `List<T>`, and
  empty dictionary literals accept concrete expected collection types from
  annotations, returns, call arguments, record fields, match arms, `if`
  branches, and constructor payloads, plus compiler-known prelude helper result
  context for callback return values. Payload-carrying ADT constructors also
  infer omitted type arguments from payload expressions when the constructor
  resolves to one visible variant and every type argument becomes concrete.
  Constructor patterns in `match` arms may constrain an otherwise unknown
  scrutinee when the visible arms identify one finite descriptor domain.
  Compiler-known collection, dictionary, option, and result helper input
  types also constrain named private callback function parameters passed to
  the implemented map, filter, fold, try-map, context-carrying dictionary
  aliases, and and-then helpers.
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
- Local inference sources, match patterns, and record pattern bindings:
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
