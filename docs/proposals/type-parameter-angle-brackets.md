# Type Parameter Angle Brackets

Status: proposed

This proposal changes source type parameter and type argument delimiters from
parentheses to angle brackets. It is not current language behavior until the
matching specification pages and executable examples say so.

## Read First

- Current source grammar and expression boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type annotation behavior:
  [../specification/types.md](../specification/types.md).
- Current ADT follow-up boundary:
  [user-defined-adts.md](user-defined-adts.md).

## Problem

Veln currently spells type constructor arguments with call-like parentheses:
`Option(Int)`, `Result(Int, E)`, `Vec(T)`, `Dict(K, V)`, `List(A)`, and
source-declared ADTs such as `Envelope(String, Error)`. Source `type`
declarations use the same delimiter for declared parameters:

```veln
pub type Envelope(A, E)
  pub Ok(A)
  pub Err(E)
end
```

That spelling makes type application visually close to value calls and ADT
variant payloads. The difference is recoverable from context, but examples,
diagnostics, and generated documentation become harder to scan when nested
types appear beside constructor calls.

## Proposed Syntax

Use angle brackets for type constructor parameters in declarations and for
type constructor arguments in type annotations:

```veln
pub type Envelope<A, E>
  pub Ok(A)
  pub Err(E)
end

pub fn parse(raw: String) -> Result<Envelope<String, Int>, ParseError>
  _
end
```

The new canonical spelling covers:

- built-in ADTs: `Option<T>` and `Result<T, E>`
- descriptor-backed ADTs: `List<T>` and source-declared ADTs
- built-in containers: `Vec<T>` and `Dict<K, V>`
- named type paths: `domain::Envelope<String, AppError>`
- nested types in function signatures, record types, contract result
  bindings, doctest metadata, generated wrapper signatures, and diagnostics

Function type parameters remain value parameter lists. Function types keep the
current function arrow shape, for example `fn(String) -> Result<(), E>`.
Variant payloads remain value-like declaration fields or tuple payloads, so
`pub Just(A)` and `Just(value)` do not change.

Type-applied call callees such as `channel::bounded[String](capacity)` are not
changed by this proposal. They are expression-level explicit type arguments,
use the existing square-bracket form, and remain limited to recognized built-in
calls unless a later proposal generalizes them.

## Parser And Formatter

Implementation should proceed in two phases.

First, accept both spellings in type positions:

- Parse `Name<Args>` and `path::Name<Args>` as the same type constructor
  application currently represented by `Name(Args)`.
- Parse `type Name<Params>` as the source-declared generic parameter list.
- Continue accepting `Name(Args)` in type positions during the compatibility
  window.
- Keep expression calls and constructor payloads unchanged.
- Treat `<` and `>` as type delimiters only while parsing a type annotation or
  type declaration parameter list. Expression parsing keeps comparison
  precedence and operator diagnostics unchanged.

Then make the formatter canonicalize type positions to angle brackets. After
that point, generated examples, doctest wrappers, human diagnostics, JSON
diagnostic fields that render expected or actual types, and documentation
snippets should use the angle-bracket spelling.

## Diagnostics

During compatibility, legacy parenthesized type arguments should remain valid.
The checker may emit a style diagnostic only if the project has a broader
style-warning route; otherwise the formatter is the migration mechanism.

After the compatibility window, parenthesized type constructor arguments in
type positions should report a parse diagnostic at the opening parenthesis. The
primary message should state the failed fact at that span, such as
``type arguments use `<...>` delimiters``. Related notes may show the canonical
spelling and explain that value calls and constructor payloads still use
parentheses.

Arity diagnostics should render the canonical type name. For example, `Dict<T>`
should say that `Dict` expects two type arguments and show `Dict<K, V>` as the
shape.

## Migration Scope

The implementation must update these surfaces together:

- grammar notes and type specification pages
- `examples/specification/` fixtures and their expected output
- standard library `.veln` sources
- parser, formatter, semantic type rendering, and doctest wrapper generation
- human and JSON diagnostic snapshots that include rendered types
- command help or generated docs that show type annotations

The migration should include checked examples for nested generic types, source
ADT declarations, built-in containers, function types returning generic
results, and comparison expressions near generic annotations so the delimiter
change does not regress expression parsing.

## Non-Goals

- Do not add higher-kinded type parameters, type constraints, traits, or
  generic functions.
- Do not change constructor expression syntax, pattern syntax, record syntax,
  dictionary syntax, vec literals, or function value call syntax.
- Do not change square-bracket explicit type arguments on built-in effect
  calls.
- Do not change runtime type representation, ADT layout, or exhaustiveness
  semantics.
- Do not document this proposal as current behavior until implementation,
  specification, and executable examples have moved together.

## Open Questions

- Should the compatibility window end in the same change that updates the
  formatter, or should the formatter canonicalize while the parser accepts the
  legacy spelling for one additional release cycle?
- Should the parser offer a targeted repair candidate from `Name(Args)` to
  `Name<Args>` after legacy spelling becomes invalid?
- Should documentation examples preserve any legacy spelling solely in negative
  tests, or should all user-facing snippets switch to the canonical spelling at
  once?

## Update When

- The parser accepts the angle-bracket form.
- The formatter chooses the canonical spelling.
- The specification and executable examples move this behavior from proposed
  to implemented.
- The legacy parenthesized spelling is removed or its compatibility boundary
  changes.
