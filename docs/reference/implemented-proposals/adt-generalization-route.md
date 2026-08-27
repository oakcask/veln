---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# ADT Generalization Route

This page records the completed ADT generalization follow-up route. Current
behavior is specified under `../../specification/` and covered by
`../../../examples/specification/`.

## Read First

- Current source grammar and constructor boundary:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current type and exhaustiveness behavior:
  [../../specification/types.md](../../specification/types.md).
- Current immutable helper behavior:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Completed list helper runtime traversal record:
  [iterative-list-helper-runtime.md](iterative-list-helper-runtime.md).
- Completed source-declared ADT follow-ups:
  [user-defined-adts.md](user-defined-adts.md).

## Implemented Outcome

`Option<T>` and `Result<T, E>` remain compiler-owned built-in ADTs.
Source-declared ADTs, `List<A>`, constructor expressions and patterns,
descriptor-backed exhaustiveness, and the implemented list helper set are
current behavior in the specification.

`Vec<A>` and `Dict<K, V>` remain built-in immutable container types from the
source user's point of view. Their helper contracts are current behavior, but
their representation and complexity are not language guarantees.

The implemented ADT boundary keeps `List<A>` and `Vec<A>` separate: no public
`list_to_vec` or `vec_to_list` helper is registered. Source code that names
those helpers reports unresolved call targets unless a user declaration with
that name is in scope.

Non-exhaustive source-declared ADT diagnostics report the first missing
constructor by its unqualified coverage case, using the constructor leaf name
and `_` for payload variants. Qualifying paths are accepted in patterns for
resolution, but the missing-case diagnostic remains a concise coverage label.

Different ADTs in the same module may expose the same constructor leaf name.
Bare use of that leaf is ambiguous, and type-qualified constructor paths remain
the disambiguation boundary.

## Evidence

- `examples/specification/check/source-adt-boundaries/` covers constructor
  namespace conflicts, same-module type-qualified disambiguation, import
  visibility, nullary generic constructor context, and the absence of public
  `List`/`Vec` conversion helpers.
- `examples/specification/check/source-adt-exhaustiveness/` covers
  source-declared ADT missing-case diagnostics.
- `docs/specification/source-surface-full.md` defines constructor visibility
  and qualified constructor paths.
- `docs/specification/types-full.md` defines ADT finite-domain exhaustiveness
  and missing-case display.
- `docs/specification/names-effects.md` defines the prelude helper set.

## Deferred Work

The source-declared ADT follow-up list is closed in
[user-defined-adts.md](user-defined-adts.md). Source-level tail-recursion
assertions or mutual-recursion guarantees remain outside this route; see
[tail-recursion-trampoline.md](tail-recursion-trampoline.md) before opening
new recursion work.

## Update When

- Current ADT, `List`, or helper behavior changes under
  `../../specification/`.
- A completed follow-up in [user-defined-adts.md](user-defined-adts.md)
  changes this boundary.
