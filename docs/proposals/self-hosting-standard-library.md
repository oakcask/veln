# Self-Hosting Standard Library

Status: `vec_map` target implemented

This page records the completed source-backed standard library target for
`vec_map` and routes later helper candidate checks. Use the specification for
current standard symbol behavior.

## Read First

- Completed helper: `vec_map`.
- Verify current source-backed versus descriptor-only status:
  [../specification/names-effects.md](../specification/names-effects.md),
  then
  [../specification/names-effects-full.md#source-backed-boundary](../specification/names-effects-full.md#source-backed-boundary).
- Confirm implemented helper signatures and value behavior:
  [helper signatures](../specification/names-effects-full.md#helper-signatures)
  and [value semantics](../specification/names-effects-full.md#value-semantics).
- Check the embedded body syntax against
  [../specification/source-surface.md](../specification/source-surface.md).
- Treat later proposal work as active only while the selected helper remains
  descriptor-only in the current specification.

## Boundary

The completed target moved `vec_map` from descriptor-only pure prelude helper
status into the existing source-backed helper model. Valid work changed only
source placement and descriptor metadata for this already implemented helper.

Later targets must not add helper semantics, effects, runtime boundaries,
parser features, module loading, source-level effect handlers, streaming,
subprocess behavior, or public container representation guarantees.
Implemented helper signatures, value semantics, and the current source-backed
split remain specification material.

## Work Route

1. Confirm the selected helper is still in the descriptor-only pure-helper
   list.
2. Keep the existing signature, value behavior, effect behavior, diagnostics
   anchoring, and backend lowering unchanged.
3. Add only the source placement and descriptor metadata needed by the existing
   source-backed helper model.
4. Open
   [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)
   only when checking candidate rules for later helpers.

## Read When

- Checking the completed `vec_map` migration or moving another
  descriptor-only helper into the source-backed pure-helper model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is implemented behavior or still
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior or helper
  semantics.
- Do not open the full proposal when the boundary and work route above answer
  the task.
