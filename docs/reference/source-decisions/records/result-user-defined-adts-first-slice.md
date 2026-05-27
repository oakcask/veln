# Discussion Result: User-Defined ADTs in the First Slice

Status: implemented

## Picked Question

- Are user-defined algebraic data types required in the first slice, or can
  `Result`, `Option`, records, lists, and dictionaries be built in initially?

## Decision

User-defined algebraic data types are not required in the first slice. The
first implementation should start with built-in `Result`, `Option`, records,
lists, dictionaries, primitive types, function types, and opaque named types
that can appear in signatures and diagnostics.

This keeps the initial checker focused on typed holes, public signatures,
`Result` propagation, contracts, effects, and structured diagnostics. Full ADT
declarations, user-defined constructors, constructor patterns, and exhaustiveness
checking should wait until examples show that built-in `Result` and `Option`
plus records are no longer enough.

## Rationale

The first slice is meant to test the repair loop, not the full expressiveness
of the language. Built-in `Result` and `Option` already exercise the most
important sum-type behavior for early examples: recoverable failure, absence,
`?` propagation, and branch-sensitive expected types for holes. Records, lists,
and dictionaries cover the common data shapes needed for configuration,
parsing, transformation, and diagnostic fixtures.

Adding user-defined ADTs immediately would bring several coupled decisions into
the prototype: declaration syntax, variant constructor syntax, payload layout,
pattern grammar, exhaustiveness diagnostics, namespace rules, type rendering,
and public API documentation. Those are valuable features, but they would widen
the first implementation before the project has proved that the smaller repair
surface works.

Opaque named types are still useful. Public APIs and diagnostics should be able
to mention names such as `UserConfig` or `ParseError` even when the first
checker does not know their constructors. This preserves readable signatures
and concrete hole messages without committing to the source syntax for defining
those types.

## First-Slice Rules

- `Result(T, E)` and `Option(T)` are built-in parametric forms with known branch
  behavior for `?`, `match`, hole expected types, and diagnostics.
- Records, lists, and dictionaries are the first user-buildable data shapes.
- Source-level `type`, `enum`, `union`, or equivalent ADT declarations are
  deferred.
- User-defined variant constructors and constructor patterns are deferred.
- The ADT first slice required exhaustiveness only for built-in `Result` and
  `Option` patterns that the first slice chose to support. Current finite
  built-in coverage, including `Bool`, is tracked in
  `../../../specification/types.md`.
- Public signatures may mention opaque named types for domain values and errors.
  The checker may compare those names nominally, render them in diagnostics, and
  use them as candidate-query targets.
- A value of an opaque named type can be produced only by already-known
  functions, built-ins, foreign bindings, or holes. The first slice does not
  invent implicit constructors for opaque names.

## Open Detail

The exact spelling of built-in variants and patterns can remain unresolved with
the broader `match` syntax. This decision only requires that `Result` and
`Option` have enough known structure for `?`, typed-hole diagnostics, and basic
branch checking.

The promotion path for ADTs should be revisited after the first examples show
which missing feature hurts the repair loop most: domain modeling, error
wrapping, exhaustiveness, generated docs, or candidate search.

## Consequence

The first checker can keep type analysis small while still producing concrete
diagnostics such as `Result(UserConfig, ParseError)` and hole queries for
`UserConfig`. Agents get useful repair targets early, and the language avoids
locking in ADT syntax before the surrounding pattern, module, and documentation
model is clearer.
