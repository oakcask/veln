# Immutable Collection Trampolines

Status: proposed

This page records the tail-recursive execution follow-up for source-authored
immutable collection helpers. It fits after the staged ADT and `List` route in
[adt-generalization-route.md](adt-generalization-route.md). It is proposal
work, not current language behavior unless `../specification/` also states it.

## Read First

- Current immutable value semantics and helper behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current execution boundary and JVM backend behavior:
  [../specification/execution.md](../specification/execution.md).
- Current source-backed helper boundary:
  [../specification/names-effects-full.md#source-backed-boundary](../specification/names-effects-full.md#source-backed-boundary).
- Staged ADT, `List`, immutable collection, and trampoline route:
  [adt-generalization-route.md](adt-generalization-route.md).
- Current source grammar, which does not include loops or mutation:
  [../specification/source-surface.md](../specification/source-surface.md).

## Current Boundary

Veln already specifies immutable container value semantics for prelude helpers.
Helpers such as `vec_push`, `vec_concat`, `vec_map`, `vec_filter`, `vec_fold`,
`vec_try_map`, and dictionary updates return new values and do not mutate their
inputs from user code. A separate ADT route proposes first removing
`Option`/`Result` special handling, then adding enough ADT support for
`List(A)`.

The current source surface has no loop syntax, reassignment, mutable local
state, method calls, or user-facing tail-call marker. Some helpers have
source-backed metadata, but the executed JVM path still reaches runtime helper
operations for collection traversal and container primitives.

A trampoline solves stack consumption for recursive helper bodies. It does not
by itself define how source code decomposes or iterates over an opaque vec or
dictionary representation. The first implementation must keep that traversal
access explicit, either by retaining a narrow runtime fold primitive as the
seed operation or by adding private compiler-admitted cursor operations for
standard-library source.

## Target

Allow immutable collection helpers to be implemented as ordinary Veln source
using structurally tail-recursive private support functions, while executing
deep traversals without consuming one host stack frame per collection element.
The preferred proving target is `List(A)` helper source after the minimal ADT
stage, not a rewrite of the existing `Vec(A)` implementation.

The first implementation strategy should be an internal trampoline for selected
standard-library helper bodies. The trampoline is a backend and runtime
strategy, not a source feature.

## Proposed Rule

Add an internal tail-call lowering path for standard-library helper functions
that are explicitly admitted by the compiler implementation. When a selected
function returns a direct call to itself or to another selected helper in tail
position, lowering may emit a trampoline step instead of a normal host call.
The runtime repeatedly evaluates steps until a final value is produced.

This does not add a `tailrec` keyword, annotation, modifier, or diagnostic
promise for user code. Ordinary user functions keep the current call behavior
unless a later proposal adds a general tail-call contract.

The trampoline must preserve the observable helper contracts for the selected
helper family:

- immutable input containers remain semantically unchanged;
- traversal order stays source order;
- `list_try_map` stops at the first `Err`;
- callback calls happen exactly where the helper semantics require them;
- diagnostics for invalid user calls remain anchored on user source spans, not
  embedded helper internals.

If a later phase applies the same route to `Vec(A)`, existing `vec_try_map` and
`vec_try_map_with` short-circuiting behavior must remain unchanged.

## Non-Goals

- Do not introduce `tailrec`, `@tailrec`, or a similar source-level marker.
- Do not promise general tail-call optimization for user functions.
- Do not add loop syntax, mutable locals, or reassignment as part of this work.
- Do not expose trampoline frames, thunks, continuation values, or helper
  implementation details in the language contract.
- Do not change current prelude helper names, signatures, value semantics, or
  effect declarations.
- Do not specify persistent vector or dictionary representation complexity.

## Work Route

1. Pick one helper family as the proving target, preferably `List(A)`
   traversal helpers after the minimal ADT stage.
2. Define the narrow tail-position pattern that the compiler accepts for
   selected standard-library helper source.
3. Choose the traversal seed. `List(A)` should use ADT pattern matching; any
   `Vec(A)` experiment still needs an existing runtime fold primitive or
   private standard-library-only cursor operations.
4. Add a backend-internal representation for trampoline steps and final values.
5. Lower selected helper tail calls to trampoline steps on the JVM backend.
6. Keep public prelude calls as ordinary helper calls from user code.
7. Add tests that execute large list traversals through public helper behavior,
   not through trampoline internals.
8. Only after implementation and tests pass, update `../specification/` with
   the implemented execution fact if it becomes observable enough to document.

## Acceptance Checks

- Large `list_fold`, `list_map`, `list_filter`, and `list_try_map` examples
  run without host stack overflow.
- Existing helper behavior tests still pass for order, immutability,
  short-circuiting, and callback invocation count.
- The checker does not accept or require any new user-facing tail-recursion
  syntax.
- A non-tail recursive selected helper body is rejected by an implementation
  check or left on the ordinary call path with a deliberate test.
- Human and JSON diagnostics for user helper misuse keep their current source
  anchors.

## Open Questions

- Should the selected-helper set be hard-coded beside standard symbol metadata,
  or derived from embedded standard-library source metadata?
- Should any `Vec(A)` helper move to trampoline-backed source in the same
  phase, or should `Vec(A)` stay on its existing runtime path until later?
- Should mutual tail recursion be admitted in the first trampoline slice, or
  should the first slice accept only direct self recursion?
- Should callback calls inside traversals be ordinary calls that may throw
  runtime failures, or should trampoline steps have a structured way to carry
  runtime failures through helper frames?
- Should a trampoline be JVM-only at first, or should the core IR carry enough
  information for future backends?

## Update When

- Trampoline lowering is implemented, rejected, or replaced with a different
  execution strategy.
- A later proposal chooses general user-facing tail-call behavior.
- The standard-library helper implementation stops needing recursion for
  collection traversal.
