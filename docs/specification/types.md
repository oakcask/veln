# Types

This page routes implemented type-system facts. Use it before opening the
full type reference.

## Read First

- Type annotations include primitives, built-in containers, records, function
  types, named type paths, and optional result bindings.
- Local inference is monomorphic and flow-sensitive within one function body.
- `Option(T)` and `Result(T, E)` are compiler-owned built-in ADTs. Their
  constructors, payload bindings, result propagation, and finite-domain
  exhaustiveness are descriptor-backed.
- `match` expressions over `Bool`, `Option(T)`, and `Result(T, E)` must be
  exhaustive unless a catch-all arm is present.
- Assignment compatibility treats `unknown` as compatible with any type and
  checks records by required fields. `Path` is distinct from `String` even
  while the current runtime stores paths with host strings.
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
