# User Function Tail Recursion Trampoline

Status: proposed target

This proposal adds a stack-safe execution route for tail-recursive
user-defined functions by lowering eligible self calls through a trampoline.
It is planned behavior only; current execution behavior remains specified under
`../specification/`.

## Read First

- Current source function and tail-expression syntax:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current execution boundary and JVM backend behavior:
  [../specification/execution.md](../specification/execution.md).
- Current function values, call resolution, and effects:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current type and function-type rules:
  [../specification/types.md](../specification/types.md).
- Completed narrow `List` helper stack-safety route:
  [../reference/implemented-proposals/iterative-list-helper-runtime.md](../reference/implemented-proposals/iterative-list-helper-runtime.md).

## Target

User-defined `fn` declarations whose self-recursive calls are all in tail
position can execute deep tail-recursive call chains without consuming one host
stack frame per recursive step.

The first target is automatic and syntax-free. A function does not need a
`tailrec` keyword or annotation. The compiler classifies eligible functions
after name resolution and type checking, then the backend lowers their
tail-position self calls to trampoline steps.

The observable function result, effect behavior, runtime `require` checks, and
existing diagnostics stay the same as ordinary execution. The new observable
guarantee is only that an eligible tail-recursive chain is host-stack safe.

## Eligibility

A function is trampoline-eligible when all of these facts hold:

- The declaration is a user-defined `fn`, not a `test` declaration.
- Every recursive call to the same source function is a direct call target that
  name resolution identifies as that function.
- Every such recursive self call appears in tail position.
- The function has no runtime return checks from `ensure` or `invariant`
  clauses in the first target.
- The function body has no reachable holes, missing expressions, constructor
  arity gaps, call arity gaps, or recognized concurrency blockers, matching
  the normal executable-IR gate.

Calls through function-typed locals, parameters, records, fields, or other
indirect values are not trampoline steps, even when a human could tell that the
value is the same function. If conservative call resolution says such an
indirect call may target the current function, the containing function is not
trampoline-eligible. Bare and `use` alias-qualified calls must keep the same
module identity rules as selected-entry reachability.

If a function is not eligible, it keeps the ordinary call lowering. The first
target does not add a warning or error for non-tail recursion.

## Tail Position

The final expression of a function body is in tail position. When that
expression is a `match`, each arm result expression is also in tail position.
This applies recursively to nested tail-position `match` expressions.

These forms are not tail position for the recursive call inside them:

- binary or prefix operators, such as `1 + loop(next)`
- call arguments, such as `wrap(loop(next))`
- record, dictionary, or vec literals
- field access or `?`
- `let` initializers
- `match` scrutinees
- non-final expression statements

For example, the recursive call in the second arm is tail position:

```veln
fn countdown(n: Int) -> Int
  match n
    0 => 0
    _ => countdown(n - 1)
  end
end
```

The recursive call here is not tail position because the caller still has work
to do after it returns:

```veln
fn length(items: List(Int)) -> Int
  match items
    List::Nil => 0
    List::Cons(_, tail) => 1 + length(tail)
  end
end
```

## Runtime Route

The typed IR remains runtime-neutral. The compiler may record trampoline
eligibility as lowering metadata, but source syntax and typed IR expression
meaning do not expose a trampoline type.

For the JVM backend, an eligible recursive self call evaluates its arguments,
packages the next parameter values as a trampoline continuation, and returns
that continuation to a backend-owned runner. The runner iterates until the
function produces a normal result. Each step enters the function boundary with
the next parameter values, so runtime `require` checks still run for each
logical recursive invocation.

The trampoline runner is a backend strategy. Its class names, allocation
strategy, continuation representation, cache keys, and helper layout are not a
source compatibility contract.

## Contracts

Runtime `require` checks are compatible with the first target because the
trampoline loop can check them at each logical function entry.

Runtime `ensure` and `invariant` return checks are not part of the first
target. Ordinary recursion checks those clauses once for every logical stack
frame after the final result returns. Preserving that behavior without host
stack growth requires a separate design, such as a heap-backed pending
postcondition list, or a narrower eligibility rule for postconditions that do
not depend on per-frame parameters.

## Non-Goals

- Do not add a `tailrec` keyword, annotation, or required source marker in the
  first target.
- Do not guarantee stack safety for non-tail recursion.
- Do not guarantee stack safety for mutual recursion in the first target.
- Do not optimize calls through function-typed values.
- Do not expose JVM trampoline classes or continuation layout as language
  behavior.
- Do not change selected-entry reachability, effect inference, call
  diagnostics, or function-value semantics.
- Do not prove termination.

## Acceptance Checks

- A deeply recursive eligible function such as `countdown` runs without host
  stack overflow through `veln run`.
- Tail-position recursive calls under nested `match` arm results are
  classified as trampoline steps.
- Non-tail recursive calls, including `1 + length(tail)`, keep ordinary call
  lowering and do not receive the stack-safety guarantee.
- A trampoline step evaluates recursive-call arguments before rebinding
  parameters for the next logical invocation.
- Runtime `require` checks execute for each logical recursive invocation.
- Functions with runtime `ensure` or `invariant` return checks are not
  classified as trampoline-eligible in the first target.
- Existing human diagnostics and command JSON for type, name, effect, and
  executable-IR blockers keep their current source anchors.
- The implemented behavior is documented under `../specification/execution.md`
  only after code and tests support it.

## Open Questions

- Should a later target add an optional `tailrec` assertion that reports an
  error when a function is not trampoline-eligible?
- Should mutually tail-recursive functions share one trampoline family after
  the direct self-recursion target is implemented?
- Should return contracts be supported by storing pending postcondition checks
  on the heap, by restricting eligible postconditions, or by keeping those
  functions outside the trampoline route?
- Should tooling expose trampoline eligibility in generated documentation or
  machine-readable analysis output?

## Update When

- Direct self-recursive trampoline lowering is implemented, rejected, or
  superseded.
- A source-level assertion, mutual-recursion route, or return-contract route is
  selected.
- Current execution behavior changes under `../specification/`.
