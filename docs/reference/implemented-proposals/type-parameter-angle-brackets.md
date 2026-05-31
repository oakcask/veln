# Type Parameter Angle Brackets

Status: implemented

This record tracks the completed migration of source type parameter and type
argument rendering from parenthesized type-constructor syntax to angle-bracket
syntax. Current behavior is specified under `../../specification/` and checked
by examples under `../../../examples/specification/`.

## Read First

- Current source grammar and expression boundary:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current type annotation behavior:
  [../../specification/types.md](../../specification/types.md).
- Current ADT follow-up boundary:
  [../../proposals/user-defined-adts.md](../../proposals/user-defined-adts.md).

## Problem

Veln previously spelled type constructor arguments with call-like parentheses:
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

## Implemented Syntax

Veln uses angle brackets for type constructor parameters in declarations and
for type constructor arguments in type annotations:

```veln
pub type Envelope<A, E>
  pub Ok(A)
  pub Err(E)
end

pub fn parse(raw: String) -> Result<Envelope<String, Int>, ParseError>
  _
end
```

The canonical spelling covers:

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

- Parse `Name<Args>` and `path::Name<Args>` as the same type constructor
  application currently represented by `Name(Args)`.
- Parse `type Name<Params>` as the source-declared generic parameter list.
- Continue accepting `Name(Args)` in type positions during the compatibility
  window.
- Keep expression calls and constructor payloads unchanged.
- Treat `<` and `>` as type delimiters only while parsing a type annotation or
  type declaration parameter list. Expression parsing keeps comparison
  precedence and operator diagnostics unchanged.
- Canonicalize type positions to angle brackets in formatter output, standard
  library sources, generated documentation, doctest wrappers, human
  diagnostics, and JSON diagnostic fields that render expected or actual
  types.

## Diagnostics

During compatibility, legacy parenthesized type arguments remain valid. The
checker does not emit a style diagnostic; the formatter is the migration
mechanism.

Arity diagnostics should render the canonical type name. For example, `Dict<T>`
should say that `Dict` expects two type arguments and show `Dict<K, V>` as the
shape.

## Completion Evidence

The implementation updated these surfaces together:

- grammar notes and type specification pages
- `examples/specification/` fixtures and their expected output
- standard library `.veln` sources
- parser, formatter, semantic type rendering, and doctest wrapper generation
- human and JSON diagnostic snapshots that include rendered types
- command help or generated docs that show type annotations

Checked examples cover nested generic types, source ADT declarations, built-in
containers, function types returning generic results, generated doctest
wrappers, and comparison expressions near generic annotations so the delimiter
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

## Follow-Up Boundary

Ending the compatibility window for parenthesized type-constructor arguments is
not part of this completed record. If that migration happens later, it should
be tracked by a new proposal that covers parse diagnostics and repair
candidates for legacy spelling.

## Update When

- The parser or formatter changes type-argument compatibility again.
- A follow-up proposal removes the parenthesized compatibility spelling.
