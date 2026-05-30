# Self-Hosting Standard Library

Status: proposed

This page routes future source-backed prelude helper migrations. It is a
proposal entry point, not the source of current helper signatures, value
semantics, or descriptor metadata.

## Read First

- Current implemented helper split, signatures, value semantics, and
  descriptor metadata:
  [../specification/names-effects.md](../specification/names-effects.md).
- Source syntax available for candidate bodies:
  [../specification/source-surface.md](../specification/source-surface.md).
- Migration pattern after one descriptor-only helper is chosen:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates).

Stop in the specification when it answers whether a helper is already
source-backed. Open the full proposal only after choosing exactly one
descriptor-only pure helper.

## Decision Route

Use the specification's source-backed boundary as the current-behavior list:

- Source-backed helper: already migrated; do not use this proposal as current
  behavior evidence.
- Descriptor-only pure helper: candidate pool for the next migration.
- Anything needing new semantics, effects, runtime boundaries, parser features,
  module loading, source-level effect handlers, streaming, subprocess behavior,
  or public container representation guarantees: out of scope.

## Work Route

1. Choose exactly one helper from the descriptor-only pure-helper list.
2. Keep its implemented signature, value behavior, effects, diagnostics
   anchoring, and backend lowering from the specification.
3. Open the full proposal's
   [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)
   section only for the candidate rule and migration pattern.
4. Add only the source placement and descriptor metadata needed by the existing
   source-backed helper model.
5. After code and tests support the move, update the implemented helper split
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
- Do not open the full proposal when the decision route above answers the task.
