---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# User-Defined ADT Follow-Ups


This record closes the source-declared ADT follow-up list. Current behavior is
specified under `../../specification/` and covered by
`../../../examples/specification/`.

## Read First

- Current source grammar and constructor boundary:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current type inference, records, and exhaustiveness behavior:
  [../../specification/types.md](../../specification/types.md).
- Current contract clause behavior:
  [../../specification/contracts.md](../../specification/contracts.md).
- ADT generalization route:
  [adt-generalization-route.md](adt-generalization-route.md).

## Implemented Outcome

Variant payload fields do not have independent visibility syntax. Constructor
visibility remains the source boundary for constructing and pattern-matching
source-declared ADTs.

Different ADTs in the same module may export constructors with the same leaf
name. Bare use of that constructor leaf is ambiguous, and type-qualified paths
select the intended ADT.

Hidden constructors remain part of the finite exhaustiveness domain. Importing
modules cannot name hidden constructors directly, so matches that cover only
public constructors must add `_` or a binding catch-all arm.

`Option`, `Result`, and `List` remain compiler-owned descriptor entries with
source-like metadata. They are not source prelude module declarations.

## Evidence

- `examples/specification/check/source-adt-boundaries/` covers same-module
  constructor leaf reuse, type-qualified disambiguation, hidden constructor
  exhaustiveness, import visibility, nullary generic constructor context, and
  the absence of public `List`/`Vec` conversion helpers.
- `examples/specification/check/source-adt-exhaustiveness/` covers
  source-declared ADT missing-case diagnostics.
- `docs/specification/source-surface-full.md` defines constructor visibility,
  payload field visibility, qualified constructor paths, and compiler-owned
  `List` constructor behavior.
- `docs/specification/types.md` defines source-declared ADT finite-domain
  exhaustiveness, hidden-constructor catch-all behavior, and missing-case
  display.

## Non-Goals

- Do not add constructor preconditions.
- Do not add type constraints, traits, deriving, methods, custom operators, or
  higher-kinded type parameters.
- Do not change `Vec`, `Dict`, or list literal behavior.
- Do not expose runtime layout as a source compatibility guarantee.
- Do not add explicit type arguments to `match`.

## Update When

- Constructor visibility, module import rules, or generic inference behavior
  changes under `../../specification/`.
- `Option`, `Result`, or `List` migrates between compiler-owned descriptors
  and source declarations.
