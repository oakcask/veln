# ADT Generalization Follow-Ups

Status: proposed follow-ups

This page keeps only ADT work that remains outside the implemented descriptor,
source-declared ADT, `List`, immutable helper, and iterative list traversal
boundary. Current behavior is specified under `../specification/` and covered
by `../../examples/specification/`.

## Read First

- Current source grammar and constructor boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type and exhaustiveness behavior:
  [../specification/types.md](../specification/types.md).
- Current immutable helper behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Completed list helper runtime traversal record:
  [../reference/implemented-proposals/iterative-list-helper-runtime.md](../reference/implemented-proposals/iterative-list-helper-runtime.md).
- Source-declared ADT follow-ups:
  [user-defined-adts.md](user-defined-adts.md).

## Current Boundary

`Option(T)` and `Result(T, E)` are compiler-owned built-in ADTs.
Source-declared ADTs, `List(A)`, constructor expressions and patterns,
descriptor-backed exhaustiveness, and the implemented list helper set are
current behavior in the specification.

`Vec(A)` and `Dict(K, V)` remain built-in immutable container types from the
source user's point of view. Their helper contracts are current behavior, but
their representation and complexity are not language guarantees.

## Proposed Follow-Ups

- Decide whether `List(A)` should expose conversion helpers to and from
  `Vec(A)` after real examples require them.
- Decide whether ADT exhaustiveness diagnostics should report missing
  constructors by qualified name, unqualified name, or both.
- Source-level tail-recursion assertions or mutual-recursion guarantees are
  outside this ADT follow-up route; see the completed user-function record in
  [../reference/implemented-proposals/tail-recursion-trampoline.md](../reference/implemented-proposals/tail-recursion-trampoline.md)
  before opening new recursion work.

## Non-Goals

- Do not change current `Vec(A)`, `Dict(K, V)`, or `[]` behavior in this
  follow-up route.
- Do not expose ADT runtime layout as a source compatibility contract.
- Do not use this page as current behavior for implemented ADT syntax,
  inference, or matching.

## Update When

- A listed follow-up is split into a narrower proposal.
- Current ADT, `List`, or helper behavior changes under `../specification/`.
