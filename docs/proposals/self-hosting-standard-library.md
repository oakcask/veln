# Self-Hosting Standard Library

Status: active `vec_try_map_with` target

This page routes the selected proposal target for moving `vec_try_map_with`
into the existing source-backed pure-helper model. It also keeps the completed
`vec_map` and `vec_try_map` migrations discoverable without making proposal
text the source of current standard symbol behavior.

## Read First

- Current target: `vec_try_map_with`.
- Current implemented source-backed versus descriptor-only status:
  [../specification/names-effects.md](../specification/names-effects.md),
  then
  [../specification/names-effects-full.md#source-backed-boundary](../specification/names-effects-full.md#source-backed-boundary).
- Implemented signature and behavior:
  [helper signatures](../specification/names-effects-full.md#helper-signatures),
  [value semantics](../specification/names-effects-full.md#value-semantics), and
  [../specification/source-surface.md](../specification/source-surface.md).
- Candidate selection and migration pattern:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates).
- Completed helper migrations: `vec_map`, `vec_try_map`.

## Boundary

The active `vec_try_map_with` target may move one already implemented pure
helper from descriptor-only status into the existing source-backed helper
model. Valid work changes only source placement and descriptor metadata after
the current code and tests support the migration.

Do not document `vec_try_map_with` as source-backed in `../specification/`
until the implementation and tests support that state. The specification stays
the source for helper signatures, value semantics, and the implemented
source-backed versus descriptor-only split.

This target must not add helper semantics, effects, runtime boundaries, parser
features, module loading, source-level effect handlers, streaming, subprocess
behavior, or public container representation guarantees.

## Work Route

1. Confirm `vec_try_map_with` is still in the descriptor-only pure-helper
   list.
2. Read its implemented signature and value behavior from the specification.
3. Open
   [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)
   only for the candidate rule and migration pattern.
4. Keep the existing signature, value behavior, effect behavior, diagnostics
   anchoring, and backend lowering unchanged.
5. Add only the source placement and descriptor metadata needed by the existing
   source-backed helper model.
6. After code and tests support the move, update the implemented helper split
   in the specification.

## Completed Helpers

`vec_map` and `vec_try_map` already moved from descriptor-only pure prelude
helper status into the source-backed helper model. Their current behavior,
source-backed status, and descriptor metadata are specification material.

## Read When

- Working on the selected `vec_try_map_with` migration.
- Checking the completed `vec_map` or `vec_try_map` migrations.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is implemented behavior or still
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior or helper
  semantics.
- Do not open the full proposal when the target, boundary, and work route above
  answer the task.
