# Self-Hosting Standard Library

Status: proposed

This page routes the remaining source-backed standard library proposal work.
Implemented standard symbols, effects, compiler-known calls, and already
source-backed helpers live in
[../specification/names-effects.md](../specification/names-effects.md).

## Current Target

Continue the source-backed pure prelude helper path. The proposal area is the
remaining descriptor-only pure helper set; already source-backed helpers are
current behavior in the specification.

Use this route for the next helper:

- confirm the implemented helper signature, value semantics, and current
  descriptor-only versus source-backed boundary:
  [../specification/names-effects-full.md#prelude-helpers](../specification/names-effects-full.md#prelude-helpers)
- confirm standard symbol descriptor metadata expectations:
  [../specification/names-effects-full.md#compiler-known-descriptor-table](../specification/names-effects-full.md#compiler-known-descriptor-table)
- confirm the current source syntax available for the embedded helper body:
  [../specification/source-surface.md](../specification/source-surface.md)
- apply the candidate filter:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)

## Read First

- Current helper behavior and source-backed boundary:
  [../specification/names-effects.md](../specification/names-effects.md).
- Remaining proposal scope and non-goals:
  [self-hosting-standard-library-full.md](self-hosting-standard-library-full.md).

## Read When

- Moving another descriptor-only helper into the source-backed pure-helper
  model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is current behavior or still future
  proposal work.

## Boundary

- Stay within pure prelude helpers whose types and lowering already fit the
  existing descriptor-backed adapter path.
- Keep broad module loading, package management, subprocess pipelines,
  streaming I/O, public complexity guarantees, and source-level effect handlers
  out of this proposal.
- Keep current behavior documented under `../specification/`; keep proposal
  wording here until the implementation and specification both support it.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior.
- Do not open the full proposal when the current target and boundary above
  answer the task.
