# User-Defined ADTs

Status: proposed

This page proposes the source surface and type-checking model for general
user-defined algebraic data types. It is proposal work, not current language
behavior unless `../specification/` also states it.

## Read First

- Current source grammar and constructor boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type inference, records, and exhaustiveness behavior:
  [../specification/types.md](../specification/types.md).
- Current contract clause behavior:
  [../specification/contracts.md](../specification/contracts.md).
- Staged ADT descriptor route:
  [adt-generalization-route.md](adt-generalization-route.md).

## Goals

- Let source modules define ordinary ADTs instead of relying on compiler-owned
  `Option`, `Result`, and the narrow `List` shape.
- Use a compact syntax close to existing type alias and call syntax.
- Support generic ADTs, recursive ADTs, tuple-like variants, and record-shaped
  variants.
- Keep type parameter scope local to the ADT declaration and require functions
  that operate over generic ADTs to declare their own type parameters.
- Make constructor visibility separate from type visibility so modules can
  preserve invariants with exported generation functions.
- Reuse existing `require` contracts for partial generation functions instead
  of adding constructor preconditions.

## Source Syntax

ADT declarations use an equals sign and pipe-separated variants:

```veln
type Option<T> = Some(T) | None

type Result<T, E> = Ok(T) | Err(E)
```

Record-shaped variants are allowed:

```veln
type Result<T, E> = Ok { value: T } | Err { error: E }
```

Recursive generic ADTs are allowed when recursion passes through a variant
payload:

```veln
type List<T> = Nil | Cons(T, List<T>)
```

The declaration introduces the type name and each variant constructor in the
declaring module. A variant constructor is a value-level function from its
payload to the ADT type. Nullary variants are zero-argument constructors.

Conceptually:

```text
Some : forall T. T -> Option<T>
None : forall T. Option<T>

Ok  : forall T, E. T -> Result<T, E>
Err : forall T, E. E -> Result<T, E>
```

Record-shaped variants conceptually take their record payload:

```text
Ok  : forall T, E. { value: T } -> Result<T, E>
Err : forall T, E. { error: E } -> Result<T, E>
```

## Generic Scope

Type parameters declared on the ADT are visible only in that ADT declaration's
right-hand side. Functions that consume or produce generic ADTs must declare
their own type parameters:

```veln
fn option_map<T, U>(input: Option<T>, f: fn(T) -> U) -> Option<U>
   match input
      Some(value) => Some(f(value))
      None => None
   end
end
```

This proposal does not add type constraints, traits, type classes, deriving,
custom operators, or higher-kinded type parameters.

## Inference

Constructor calls instantiate the constructor's generic type independently at
the call site.

`Some(1)` infers `Option<Int>` by unifying the constructor payload type with
`Int`.

`None` initially has type `Option<A>` for a fresh unresolved type variable.
The checker must resolve that variable from assignment, return, call, match, or
other surrounding context. If no context resolves it, inference fails:

```veln
let a: Option<Int> = None
let b = Some(1)
let c = None
```

The first two bindings are accepted. The last binding is rejected because the
element type of `Option` is not determined.

The same rule applies to nullary constructors with unused type parameters in
other ADTs.

## Name Resolution

A declaration inside a module introduces both module-qualified and
type-qualified constructor paths:

```veln
mod core

type Option<T> = Some(T) | None
```

Visible paths include:

- `core::Option`
- `core::Some`
- `core::None`
- `core::Option::Some`
- `core::Option::None`

When a module imports `core`, unqualified constructor names are available if
they are not ambiguous:

```veln
mod app

use core

fn foo() -> ()
   let a = Some(1)
   let b = Option::Some(1)
   let c = core::Some(1)
   let d = core::Option::Some(1)
end
```

If two imports expose the same unqualified constructor name, the unqualified
use is ambiguous and must be rejected:

```veln
mod app

use core
use core2

fn foo() -> ()
   let a = Some(1)
   let b = core::Some(1)
end
```

The `Some(1)` binding is rejected when both imports export `Some`. The
qualified binding is accepted.

Within one module, two exported or unexported constructors must not share the
same constructor name. This keeps a module from defining an always-ambiguous
export surface.

## Visibility

Type visibility and constructor visibility are separate.

Public type plus public default constructor:

```veln
pub type Rect = pub { x: Int, y: Int, w: Int, h: Int }
```

The module exports both the `Rect` type and the `Rect { ... }` constructor.

Public type plus private default constructor:

```veln
pub type Rect = { x: Int, y: Int, w: Int, h: Int }
```

The module exports the `Rect` type but does not export the `Rect { ... }`
constructor. Code in the declaring module can still construct the value.

For ADT variants, `pub` on a variant exports that variant constructor:

```veln
pub type Shape =
   pub Circle { center: Point, radius: Int }
 | Rectangle(Rect)
```

`Circle` is exported; `Rectangle` is available only inside the declaring
module.

Field access on immutable values remains allowed when the field type is known.
Pattern destructuring outside the declaring module follows constructor
visibility: external code cannot match on private constructors because that
would expose the hidden representation.

## Generation Functions

Constructors only build values. They do not contain preconditions or
validation clauses. Modules that need invariants should keep constructors
private and export generation functions.

Use the existing `require` contract for partial generation functions where
invalid arguments are caller contract violations:

```veln
pub type Rect = { x: Int, y: Int, w: Int, h: Int }

pub fn rect(x: Int, y: Int, w: Int, h: Int) -> Rect
require w >= 0 and h >= 0
   Rect { x, y, w, h }
end
```

Use `Result` for total validation functions that accept unchecked input:

```veln
pub fn try_rect(x: Int, y: Int, w: Int, h: Int) -> Result<Rect, String>
   if w < 0
      Err("width must be non-negative")
   else if h < 0
      Err("height must be non-negative")
   else
      Ok(Rect { x, y, w, h })
   end
end
```

## Pattern Matching

`match` does not accept explicit type arguments. Constructor payload bindings
come from the scrutinee type and the matched variant descriptor.

Finite-domain exhaustiveness extends from compiler-owned descriptors to every
source-declared ADT whose variant set is known. A match over `Option<T>` must
cover `Some(_)` and `None`, a match over `Result<T, E>` must cover `Ok(_)` and
`Err(_)`, and a match over `List<T>` must cover `Nil` and `Cons(_)`, unless a
catch-all arm is present.

Private constructors still count for exhaustiveness inside the declaring
module. Outside the declaring module, matching a type with hidden constructors
requires a catch-all arm unless a later specification adds an explicit
non-exhaustive or opaque-match rule.

## Compatibility And Migration

`Option`, `Result`, and `List` should be expressible through this general ADT
model. The migration target is to reduce compiler special cases while
preserving current behavior:

- constructor calls and patterns keep their existing source spellings;
- `Result` propagation with `?` keeps using `Result` metadata;
- diagnostics keep reporting at the same user spans;
- finite-domain exhaustiveness keeps the same behavior for existing built-in
  and minimal source-declared cases.

## Non-Goals

- Do not add constructor preconditions.
- Do not add type constraints, traits, deriving, methods, or custom operators.
- Do not change `Vec`, `Dict`, or list literal behavior.
- Do not expose runtime layout as a source compatibility guarantee.
- Do not add explicit type arguments to `match`.

## Acceptance Checks

- Source can declare `Option<T>`, `Result<T, E>`, and `List<T>` with the
  proposed syntax.
- Constructor calls infer generic result types from payloads.
- Nullary generic constructors require surrounding type context.
- Recursive ADTs through variant payloads are accepted.
- Constructor name conflicts in one module are rejected.
- Ambiguous unqualified constructor imports are rejected.
- Qualified constructor paths resolve through module and type paths.
- Public types can hide constructors from external modules.
- Private constructors remain usable inside their declaring module.
- Generation functions can enforce invariants with `require` or return
  `Result`.
- Exhaustiveness checks use source-declared variant sets.

## Open Questions

- Should a later proposal add field visibility as a third independent axis
  after the initial public-read field model proves insufficient?
- Should a module be allowed to export two constructors with the same leaf name
  if both are only usable through type-qualified paths?
- Should hidden constructors make external exhaustive matches impossible, or
  should the language expose a separate opaque-match rule?
- Should standard `Option`, `Result`, and `List` eventually live in source
  prelude modules, or remain compiler-owned descriptor entries with source-like
  metadata?

## Update When

- General user-defined ADTs become current behavior under `../specification/`.
- Constructor visibility, module import rules, or generic inference behavior is
  implemented differently from this proposal.
- `Option`, `Result`, or `List` migrate between compiler-owned descriptors and
  source declarations.
