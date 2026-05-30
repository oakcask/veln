# Self-Hosting Standard Library

Status: proposed

This page routes future source-backed prelude helper migrations. It is a
proposal entry point, not the source of current helper signatures, value
semantics, or descriptor metadata.

## Read First

- Current helper split, candidate pool, signatures, value semantics, and
  descriptor metadata:
  [../specification/names-effects.md](../specification/names-effects.md).
- Direct candidate boundary:
  [../specification/names-effects-full.md#source-backed-boundary](../specification/names-effects-full.md#source-backed-boundary).
- Source syntax available for candidate bodies:
  [../specification/source-surface.md](../specification/source-surface.md).
- Candidate and migration rule after one descriptor-only helper is chosen:
  [self-hosting-standard-library-full.md#candidate-and-migration-rule](self-hosting-standard-library-full.md#candidate-and-migration-rule).

## Target Route

1. Open the specification's source-backed boundary to find the live
   descriptor-only pure-helper candidate pool.
2. Stop there when the helper is already source-backed.
3. Choose exactly one helper from that pool.
4. Check the full proposal's candidate and migration rule.
5. Add only the source placement and descriptor metadata needed by the current
   source-backed helper model.
6. After code and tests support the move, update the implemented helper split
   in the specification before treating the behavior as current.

## Scope Checks

- Keep implemented signatures, value behavior, effects, diagnostics anchoring,
  and backend lowering in the specification.
- Treat descriptor-only pure helpers as the next candidate pool.
- Leave work that needs new semantics, effects, runtime boundaries, parser
  features, module loading, source-level effect handlers, streaming,
  subprocess behavior, or public container representation guarantees out of
  this proposal.

## Read When

- Choosing the next descriptor-only pure-helper candidate.
- Routing a standard-library idea between implemented behavior and future
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior, helper
  classification, or helper semantics.
- Do not open the full proposal before one descriptor-only helper has been
  selected.
