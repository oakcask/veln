# Self-Hosting Standard Library

Status: proposed

This page records completed prelude helper migrations and routes future
source-backed candidates back through the implemented standard symbol split.
Proposal text here is not the source of current helper signatures, value
semantics, or descriptor metadata.

## Read First

- Current implemented helper split, signatures, value semantics, and
  descriptor metadata:
  [../specification/names-effects.md](../specification/names-effects.md),
  then
  [source-backed boundary](../specification/names-effects-full.md#source-backed-boundary),
  [helper signatures](../specification/names-effects-full.md#helper-signatures),
  and
  [value semantics](../specification/names-effects-full.md#value-semantics).
- Source syntax available for candidate bodies:
  [../specification/source-surface.md](../specification/source-surface.md).
- Candidate rule and migration pattern after one descriptor-only helper is
  chosen:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates).

Choose exactly one descriptor-only pure helper before promoting future helper
work into a concrete target.

## Current Boundary

`vec_map`, `vec_try_map`, and `vec_try_map_with` already moved from
descriptor-only pure prelude helper status into the source-backed helper model.
Their current behavior, source-backed status, and descriptor metadata are
specification material.

The specification stays the source for helper signatures, value semantics, and
the implemented source-backed versus descriptor-only split.

Future targets must not add helper semantics, effects, runtime boundaries,
parser features, module loading, source-level effect handlers, streaming,
subprocess behavior, or public container representation guarantees.

## Work Route

1. Choose exactly one helper from the descriptor-only pure-helper list.
2. Read its implemented signature and value behavior from the specification.
3. Open
   [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)
   only for the candidate rule and migration pattern.
4. Keep the existing signature, value behavior, effect behavior, diagnostics
   anchoring, and backend lowering unchanged.
5. Add only the source placement and descriptor metadata needed by the existing
   source-backed helper model.
6. After code and tests support the move, update the implemented helper split
   in the specification before treating the behavior as current.

## Read When

- Checking completed prelude helper migrations.
- Choosing the next descriptor-only pure-helper candidate.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is implemented behavior or still
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior or helper
  semantics.
- Do not open the full proposal when the target, boundary, and work route above
  answer the task.
