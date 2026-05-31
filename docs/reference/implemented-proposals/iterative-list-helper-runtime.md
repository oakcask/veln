# Iterative List Helper Runtime

Status: implemented

This page records the completed list helper traversal follow-up. The
implemented route uses iterative JVM runtime support for public `List<A>`
helpers instead of adding source helper trampoline lowering.

## Read First

- Current immutable value semantics and helper behavior:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Current execution boundary and JVM backend behavior:
  [../../specification/execution.md](../../specification/execution.md).
- Current source-backed helper boundary:
  [../../specification/names-effects-full.md#source-backed-boundary](../../specification/names-effects-full.md#source-backed-boundary).
- Completed ADT, `List`, immutable collection, and traversal runtime route:
  [adt-generalization-route.md](adt-generalization-route.md).
- Current source grammar, which does not include loops or mutation:
  [../../specification/source-surface.md](../../specification/source-surface.md).

## Implemented Outcome

The implemented proving target is the `List<A>` helper family. The helpers are
source-backed in `prelude`, and public JVM calls for `list_fold`,
`list_reverse`, `list_map`, `list_filter`, and `list_try_map` execute through
runtime support that traverses the list representation iteratively instead of
lowering source helper recursion to trampoline steps.

No user-facing `tailrec` syntax, annotation, diagnostic contract, or general
tail-call optimization guarantee was added. The current observable behavior is
documented in the specification execution and prelude helper pages.

## Original Target

The original proposal target was to allow immutable collection helpers to be
implemented as ordinary Veln source using structurally tail-recursive private
support functions, while executing deep traversals without consuming one host
stack frame per collection element. The preferred proving target was `List<A>`
helper source after the minimal ADT stage.

The original proposal named this work as a trampoline route. The implementation
replaced the internal trampoline lowering slice with a narrow runtime strategy.
This keeps the observable acceptance boundary while avoiding new
compiler-admitted tail-call syntax or selected-helper checks.

## Acceptance Review

- Large `list_fold`, `list_map`, `list_filter`, and `list_try_map` calls run
  through JVM runtime support without host stack overflow.
- Existing helper behavior is covered for order, immutability,
  short-circuiting, and callback invocation count.
- The checker does not accept or require new user-facing tail-recursion syntax.
- There is no selected-helper trampoline lowering path, so non-tail recursive
  selected-helper acceptance is not part of the implemented strategy.
- Human and JSON diagnostics for user helper misuse keep their current source
  anchors because public helper calls still use descriptor-backed checking.

## Evidence

- The JVM runtime source fragments implement list traversal helpers with loops
  over `ListValue`.
- `crates/veln-backend-jvm/src/tests.rs` covers runtime and public helper
  traversal over large lists.
- `examples/specification/run/prelude-containers/` covers public list helper
  value semantics in the executable specification examples.

## Update When

- General user-facing tail-call behavior is proposed or implemented.
- `List<A>` helper names, signatures, value semantics, or runtime traversal
  strategy change.
