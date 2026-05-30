# ADT Generalization Route

Status: stage 1 implemented; stages 2 and 3 proposed

This page records a staged route from compiler-special `Option` and `Result`
handling toward user-defined ADTs, standard-library `List`, immutable
collection helpers, and internal tail-recursive execution support. It is
proposal work, not current language behavior unless `../specification/` also
states it.

## Read First

- Current source grammar and constructor boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type and exhaustiveness behavior:
  [../specification/types.md](../specification/types.md).
- Current immutable helper behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Tail-recursive helper execution follow-up:
  [immutable-collection-trampoline.md](immutable-collection-trampoline.md).

## Current Boundary

`Option(T)` and `Result(T, E)` are implemented built-in parametric forms.
Their constructors, patterns, branch typing, `?` behavior, and exhaustiveness
rules now route through compiler-owned ADT descriptors. User-defined ADT
declarations and user-defined constructors are not implemented.

`Vec(A)` and `Dict(K, V)` remain built-in immutable container types from the
source user's point of view. Their helper contracts are current behavior, but
their representation and complexity are not language guarantees.

## Target Sequence

Use three implementation stages instead of introducing `List`, immutable
collection source helpers, and tail-call recursion in one step.

### Stage 1: Descriptor-Backed Option And Result

Status: implemented

Replace scattered `Option` and `Result` special cases with a compiler-owned
ADT descriptor model. This stage should preserve all current source behavior.

The descriptor model should represent:

- type constructor name and generic parameters;
- variant constructor names and payload fields;
- constructor arity and qualified names;
- pattern coverage used for exhaustiveness;
- branch typing for constructor patterns;
- `Result` propagation metadata needed by `?`.

This stage should not add user-declared ADT syntax. The acceptance target is
that `Option` and `Result` behavior is still identical while the checker,
lowering, and diagnostics ask the descriptor table for facts instead of using
separate hard-coded branches wherever practical.

### Stage 2: Minimal ADTs For List

Status: proposed

Extend the descriptor model into a narrow source feature only far enough to
define `List(A)` and pattern match on it.

The first source-declared ADT target should support this shape:

```text
type List(A)
  Nil
  Cons(head: A, tail: List(A))
end
```

The required language surface is:

- recursive generic ADT declarations;
- nullary and product variants;
- constructor expressions and constructor patterns;
- nominal assignment compatibility for ADT names;
- exhaustiveness over the declared finite variant set;
- stable constructor namespace rules for unqualified and qualified names.

This stage should leave `Vec(A)` unchanged. The list literal syntax `[]`
continues to mean the existing vec literal unless a later proposal changes it.

### Stage 3: List Helpers And Tail-Recursive Execution

Status: proposed

After `List(A)` is expressible, add standard-library list helpers and use them
as the proving target for internal tail-recursive execution.

Candidate helpers:

- `list_nil`;
- `list_cons`;
- `list_is_empty`;
- `list_fold`;
- `list_reverse`;
- `list_map`;
- `list_filter`;
- `list_try_map`.

`List` helpers should preserve immutable value semantics and source-order
traversal. Tail-recursive helper bodies may use the internal trampoline route
described in
[immutable-collection-trampoline.md](immutable-collection-trampoline.md), but
that remains a compiler/runtime strategy, not a user-facing `tailrec` feature.

## Non-Goals

- Do not introduce a `tailrec` keyword, annotation, or user-facing tail-call
  guarantee.
- Do not change `Vec(A)`, `Dict(K, V)`, or `[]` behavior in the ADT stage.
- Do not require methods, traits, deriving, custom operators, or type classes.
- Do not add broad generic inference beyond what the existing type system and
  `List(A)` need.
- Do not promise persistent vector, dictionary, or list complexity classes.
- Do not expose ADT runtime layout as a source compatibility contract.

## Acceptance Checks

- Stage 1 preserves current `Option` and `Result` constructor, pattern,
  exhaustiveness, and `?` behavior with existing tests.
- Stage 1 has focused tests proving diagnostics still use the same user source
  anchors after descriptor routing.
- Stage 2 can declare `List(A)` with `Nil` and `Cons(A, List(A))`.
- Stage 2 can type-check and run pattern matches over `List(A)`.
- Stage 2 rejects non-exhaustive `List(A)` matches with a diagnostic shaped
  like current finite-domain match diagnostics.
- Stage 3 list helpers pass behavior tests for order, immutability,
  short-circuiting, and callback invocation count.
- Stage 3 large list traversals run without host stack overflow without adding
  source-level tail-recursion syntax.

## Open Questions

- Should `Option` and `Result` remain built-in descriptor entries forever, or
  eventually become declarations in a standard prelude source unit?
- Should `List(A)` live in the same prelude namespace as `Vec(A)` helpers, or
  behind an explicit module once module exports are richer?
- Should constructor names such as `Nil` and `Cons` be globally reserved,
  module-qualified, or imported like ordinary values?
- Should `List(A)` expose conversion helpers to and from `Vec(A)` in the first
  list stage, or wait until real examples require them?
- Should ADT exhaustiveness diagnostics report missing constructors by
  qualified name, unqualified name, or both?

## Update When

- The descriptor-backed `Option` and `Result` migration is implemented or
  rejected.
- Source-declared ADTs become current behavior under `../specification/`.
- `List(A)` helpers are implemented, renamed, or scoped differently.
- Tail-recursive execution support moves into the implemented execution
  reference.
