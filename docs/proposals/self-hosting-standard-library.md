# Self-Hosting Standard Library

Status: proposed

This page routes future source-backed prelude helper migrations through the
implemented standard symbol split. Proposal text here is not the source of
current helper signatures, value semantics, or descriptor metadata.

## Read First

- Current implemented helper split, signatures, value semantics, and
  descriptor metadata:
  [../specification/names-effects.md](../specification/names-effects.md).
- Source syntax available for candidate bodies:
  [../specification/source-surface.md](../specification/source-surface.md).
- Candidate rule and migration pattern after one descriptor-only helper is
  chosen:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates).

Choose exactly one descriptor-only pure helper before promoting future helper
work into a concrete target.

## Current Boundary

The specification is the only current-behavior list for source-backed versus
descriptor-only pure helpers. A helper in the specification's source-backed
list is already migrated; a helper in its descriptor-only pure-helper list is
the remaining candidate pool for this proposal.

Future targets must not add helper semantics, effects, runtime boundaries,
parser features, module loading, source-level effect handlers, streaming,
subprocess behavior, or public container representation guarantees.

## Work Route

1. Choose exactly one helper from the specification's descriptor-only
   pure-helper list.
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

- Checking whether a prelude helper migration is already complete.
- Choosing the next descriptor-only pure-helper candidate.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is implemented behavior or still
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior, helper
  classification, or helper semantics.
- Do not open the full proposal when the target, boundary, and work route above
  answer the task.
