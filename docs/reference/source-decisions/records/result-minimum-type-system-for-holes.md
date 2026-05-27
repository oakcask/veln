# Discussion Result: Minimum Type System for Holes

Status: implemented

## Picked Question

- What is the minimum type system that can support useful typed holes without
  making the first implementation too large?

## Decision

The first slice should use a small, local, mostly monomorphic type system with
built-in parametric forms for `Option`, `Result`, lists, dictionaries, and
function types. It should infer private helper and expression types through
local unification, but it should not include user-defined generics, traits,
type classes, subtyping, implicit conversions, or generalized let-polymorphism.

This is enough for typed holes when the checker can derive expected types from
explicit public signatures, annotated local bindings, call arguments, declared
return positions, record fields, collection elements, `match` branches,
contracts, and `?` propagation.

## Rationale

Typed holes are useful only when diagnostics can say what value is missing and
why. The first implementation does not need a powerful language-wide type
system to do that; it needs predictable expected-type flow through common
program shapes.

Public function signatures already give the checker stable boundary facts.
Local unification can then push those facts inward to holes without requiring a
large inference engine. Built-in parametric forms are worth including because
they appear in the proposed first slice and directly improve repair hints:
`Result(User, ParseError)` is much more actionable than an opaque fallible
value, and `Vec(User)` gives candidate search better shape than an unknown
collection.

Avoiding generalized polymorphism keeps diagnostics simpler. A private helper
can still be inferred for one concrete use, but reusable generic behavior
should wait until examples show that the repair loop needs it. This reduces
the chance that first-slice hole output becomes dominated by abstract type
variables instead of concrete edit guidance.

## First-Slice Rules

- Primitive types include at least `Bool`, `Int`, `Float`, `String`, and
  `()`.
- Built-in compound types include records, homogeneous lists, homogeneous
  dictionaries, function types, `Option(T)`, and `Result(T, E)`.
- Type variables may exist internally during checking, but public diagnostics
  should render them only when no concrete expected type can be derived.
- Private function inference is local and monomorphic by default. The checker
  may infer a helper type from its body and use sites, but it does not
  generalize that type for unrelated call sites.
- A hole should receive the most specific expected type available from its
  expression context. If no useful type can be derived, report
  `expected_type: "unknown"` and include the closest missing-context reason.
- `?` contributes expected-type information by requiring the surrounding
  function or anonymous function to return a compatible `Result`.
- Contracts may refine repair context, but they do not replace the static type
  expected for a hole.

## Open Detail

The first-slice grammar resolves the initial source spelling for type
arguments as `Result(T, E)`, `Option(T)`, and other `TypePath(...)` forms.
Future generic syntax can revisit that spelling if user-defined generics need a
different shape.

User-defined algebraic data types and error type inference remain separate
open questions. This decision only says the first typed-hole implementation can
be useful before those features exist.

## Consequence

The first checker can produce actionable hole diagnostics without committing
to a large type system. Agents get concrete expected types for the common
repair path, while the language keeps enough design space for later generics,
user-defined data types, and richer error inference.
