# Self-Hosting Standard Library

Status: proposed

This page routes the active source-backed standard library proposal work. Use
it only after [../specification/names-effects.md](../specification/names-effects.md)
shows that the helper is still descriptor-only.

## Current Target

Move one remaining descriptor-only pure prelude helper into the existing
source-backed helper model. The proposal changes helper source placement and
descriptor metadata only; implemented signatures, value semantics, and the
current source-backed split remain specification material.

## Decision Route

1. Confirm the helper is in the descriptor-only pure-helper list in
   [../specification/names-effects.md](../specification/names-effects.md).
2. Confirm its implemented signature and value semantics on that same
   specification route.
3. Check [../specification/source-surface.md](../specification/source-surface.md)
   for the source forms needed by the embedded body.
4. Use
   [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)
   only for migration mechanics and future-work boundaries.

## Candidate Guardrails

A valid target changes only source placement and descriptor metadata for one
already implemented descriptor-only pure helper. It must not require new
effects, runtime boundaries, parser features, module loading, source-level
effect handlers, streaming, subprocess behavior, or public container
representation guarantees.

## Read When

- Moving another descriptor-only helper into the source-backed pure-helper
  model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is current behavior or still future
  proposal work.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior or helper
  semantics.
- Do not open the full proposal when the current target and guardrails
  above answer the task.
