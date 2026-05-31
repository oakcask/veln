# User-Defined ADT Follow-Ups

Status: proposed follow-ups

This page keeps only source-declared ADT work that remains outside the
implemented language surface. Current ADT syntax, constructor visibility,
generic inference, name resolution, and exhaustiveness behavior are specified
under `../specification/` and covered by `../../examples/specification/`.

## Read First

- Current source grammar and constructor boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type inference, records, and exhaustiveness behavior:
  [../specification/types.md](../specification/types.md).
- Current contract clause behavior:
  [../specification/contracts.md](../specification/contracts.md).
- Completed ADT generalization route:
  [../reference/implemented-proposals/adt-generalization-route.md](../reference/implemented-proposals/adt-generalization-route.md).

## Proposed Follow-Ups

- Decide whether field visibility should become independent from type and
  constructor visibility after the current public-read field model proves
  insufficient.
- Decide whether a module may export two constructors with the same leaf name
  when both are only usable through type-qualified paths.
- Decide whether hidden constructors should make external exhaustive matches
  impossible or whether the language should expose an explicit opaque-match
  rule.
- Decide whether standard `Option`, `Result`, and `List` eventually live in
  source prelude modules or remain compiler-owned descriptor entries with
  source-like metadata.

## Non-Goals

- Do not add constructor preconditions.
- Do not add type constraints, traits, deriving, methods, custom operators, or
  higher-kinded type parameters.
- Do not change `Vec`, `Dict`, or list literal behavior.
- Do not expose runtime layout as a source compatibility guarantee.
- Do not add explicit type arguments to `match`.

## Update When

- A listed follow-up is split into a narrower proposal.
- Constructor visibility, module import rules, or generic inference behavior
  changes under `../specification/`.
- `Option`, `Result`, or `List` migrates between compiler-owned descriptors
  and source declarations.
