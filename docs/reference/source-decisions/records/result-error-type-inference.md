# Discussion Result: Error Type Inference

Status: implemented

## Picked Question

- How should error type inference work when several fallible operations appear
  in one function?

## Decision

The first slice should not infer open-ended error unions across a function.
Instead, public and exported functions must write the error type in their
explicit `Result` return type, and every `?` in the function must produce an
error that is already compatible with that declared type.

Private helpers may infer a concrete error type when all fallible operations
produce the same error type. If multiple incompatible error types appear, the
checker should require an explicit conversion at the operation site or an
explicit return type on the helper. The first slice may support built-in
conversion through a small, explicit form, but it should not silently widen
errors into an anonymous union.

## Rationale

Error flow is part of the agent-facing repair surface. When a function contains
several fallible operations, silent widening makes the generated API harder to
review: the caller must discover the possible failures by reading the body,
and diagnostics for a missing conversion become less local.

The existing public-boundary rule already requires explicit return types on
public functions. Reusing that boundary for errors keeps API contracts visible
without requiring a full algebraic data type system in the first slice. It also
gives `?` a simple check: does this operation's error fit the function's
declared `Result` error?

Private helpers can stay lightweight when the answer is obvious. A helper that
only propagates `ParseError` can infer `Result(T, ParseError)`. Once a helper
mixes `ParseError` and `IoError`, the implementation should force the author or
agent to choose the intended public shape, such as converting both into
`ConfigError`.

Avoiding anonymous inferred unions keeps diagnostics actionable. The checker
can point at the exact `?` whose error does not match and suggest adding a
conversion or declaring a broader named error type later, instead of presenting
a synthetic type that may not be stable across edits.

## First-Slice Rules

- Public functions that return `Result` must explicitly name both the success
  and error type.
- `?` is valid only when the current function or anonymous function returns a
  compatible `Result`.
- If the surrounding return error type is known, each `?` must either produce
  that type or use an explicit conversion accepted by the checker.
- Private helpers may infer `Result(T, E)` when every propagated fallible
  operation has the same concrete `E`.
- Private helpers with multiple incompatible propagated errors must receive an
  explicit return type or explicit conversions before inference succeeds.
- The first slice should not synthesize anonymous error union types as public
  diagnostic output.

## Open Detail

The exact conversion syntax can remain unresolved. It may become a function
call, a `map_error`-style helper, or a dedicated operator. This decision only
requires that conversion be explicit and local enough for diagnostics to point
at the responsible fallible operation.

User-defined algebraic data types remain a separate question. Until they exist,
the implementation can model named error types opaquely or provide built-in
error wrapper examples for tests.

## Consequence

The first checker can keep `?` behavior predictable while still supporting
fallible programs with more than one operation. Agents get local repair hints
for error mismatches, public APIs keep visible error contracts, and the
language avoids committing early to a broad union or effect-style error system.
