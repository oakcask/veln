# Self-Hosting Standard Library

Status: `vec_try_map` target implemented

This page records the completed source-backed standard library targets for
`vec_map` and `vec_try_map` and routes later helper candidate checks. Use the
specification for current standard symbol behavior.

## Read First

- Completed helpers: `vec_map`, `vec_try_map`.
- Verify current source-backed versus descriptor-only status:
  [../specification/names-effects.md](../specification/names-effects.md),
  then
  [../specification/names-effects-full.md#source-backed-boundary](../specification/names-effects-full.md#source-backed-boundary).
- Read implemented signature and behavior only from the specification:
  [helper signatures](../specification/names-effects-full.md#helper-signatures),
  [value semantics](../specification/names-effects-full.md#value-semantics), and
  [../specification/source-surface.md](../specification/source-surface.md).
- Open the full proposal only for candidate selection rules:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates).

## Boundary

The completed targets moved `vec_map` and `vec_try_map` from descriptor-only
pure prelude helper status into the existing source-backed helper model. Valid
work changed only source placement and descriptor metadata for already
implemented helpers.

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

- Checking the completed `vec_map` or `vec_try_map` migrations, or moving
  another descriptor-only helper into the source-backed pure-helper model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is implemented behavior or still
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior or helper
  semantics.
- Do not open the full proposal when the boundary and work route above answer
  the task.
