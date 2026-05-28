# Self-Hosting Standard Library

Status: proposed

This page routes source-backed standard library proposal work whose behavior is
absent from the current specification. Implemented standard symbols, effects,
and compiler-known calls live in
[../specification/names-effects.md](../specification/names-effects.md).

## Read First

- Current implemented names, prelude helper signatures, source-backed helper
  status, stdio, file-system, process, and effect behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current source surface:
  [../specification/source-surface.md](../specification/source-surface.md).
- Remaining source-backed standard library proposal details:
  [self-hosting-standard-library-full.md](self-hosting-standard-library-full.md).

## Read When

- Moving another descriptor-only helper into the source-backed pure-helper
  model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.

## Boundary

- Stay within pure prelude helpers whose types and lowering already fit the
  existing descriptor-backed adapter path.
- Keep broad module loading, package management, subprocess pipelines,
  streaming I/O, public complexity guarantees, and source-level effect handlers
  out of this proposal.
- Keep current behavior documented under `../specification/`; keep proposal
  wording here until the implementation and specification both support it.

## Open Questions

- Which descriptor-only pure helper should follow `option_map` after the
  source-backed pattern is validated?
- What later source-level effect abstraction should replace or wrap coarse
  built-in effect labels?
- Should process termination use the current return types or a future never
  type?

## Skip Unless Needed

- Do not use this page for current standard symbol behavior.
- Do not open the full proposal when the short boundary above answers the task.
