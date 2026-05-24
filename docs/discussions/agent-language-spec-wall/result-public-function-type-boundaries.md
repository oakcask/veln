# Discussion Result: Public Function Type Boundaries

## Picked Question

- Should public functions require explicit parameter and return types from the
  beginning?

## Decision

Require explicit parameter and return types on public functions from the first
slice. Allow private helper functions to use inference unless their inferred
types need to appear in public diagnostics, generated docs, or exported module
metadata.

## Rationale

Public functions are the highest-value boundary for agents. They are the places
where generated code must interoperate with tests, callers, documentation,
contracts, and later modules. If public signatures are inferred, an agent must
read more implementation detail before it can safely call, refactor, or repair
the function. Requiring signatures at public boundaries keeps the review surface
small and gives typed holes better expected-type context.

Private helpers are different. They often exist only to make one implementation
local and readable. Requiring all helper signatures early would increase
ceremony before the type system and formatter have proved their shape. Private
inference also lets examples stay short while the first slice is still testing
the core repair loop.

## First-Slice Rule

- Any exported function must write every parameter type and its return type.
- A missing public type annotation is a `type.public_signature_missing`
  diagnostic.
- Private functions may omit annotations when inference can produce a complete
  type.
- If inference for a private function fails or leaks an unresolved type into a
  public signature, report the diagnostic at the public boundary and include the
  private helper as related context.
- Contracts on public functions are checked against the explicit signature, not
  used as a substitute for it.

## Open Detail

The exact public marker can wait for the syntax discussion. It may be `pub fn`,
an export list, package metadata, or another module mechanism. The policy is
that once a function is part of a public API, its full signature is explicit.

## Consequence

The first implementation can keep inference local while still making generated
APIs stable enough for agents, doctests, JSON diagnostics, and future module
documentation.
