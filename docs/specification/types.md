# Types

This page routes implemented type-system facts. Use it before opening the
full type reference.

## Read First

- Type annotations include primitives, descriptor-backed `Option`, `Result`,
  `List`, source-declared ADTs, built-in containers, records, function types,
  named type paths, and optional result bindings. Type constructor arguments
  use angle brackets; parenthesized type constructor arguments are rejected in
  source type positions.
- Local inference is monomorphic and flow-sensitive within one function body.
- `Option<T>` and `Result<T, E>` are compiler-owned built-in ADTs. `List<T>`
  and source-declared ADTs are descriptor-backed. Their constructors, payload
  bindings, result propagation where applicable, and finite-domain
  exhaustiveness use descriptor facts.
- The standard prelude exposes source-visible `Byte`, `ByteChunk`,
  `ByteOffset`, and `ByteCount` named types for small immutable byte values
  and byte-counted helper APIs.
- `match` expressions over `Bool`, `Option<T>`, `Result<T, E>`, `List<T>`, and
  source-declared ADTs must be exhaustive unless a catch-all arm is present.
- Assignment compatibility treats `unknown` as compatible with any type and
  checks records by required fields. `Path` is distinct from `String`; the
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
