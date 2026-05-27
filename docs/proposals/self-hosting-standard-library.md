# Self-Hosting Standard Library

Status: proposed

This page tracks future self-hosting standard library questions whose behavior
is absent from the current specification. Implemented standard symbols, effects,
and compiler-known calls live in
[../specification/names-effects.md](../specification/names-effects.md).

## Read First

- Current names, prelude, stdio, file-system, process, and effect behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Current source surface:
  [../specification/source-surface.md](../specification/source-surface.md).
- Full future-work notes:
  [self-hosting-standard-library-full.md](self-hosting-standard-library-full.md).

## Open Questions

- Which additional reusable helpers should move from compiler-known descriptors
  into source-backed library modules?
- What source-level effect abstraction should replace coarse built-in effect
  labels?
- Should process termination use the current return types or a future never
  type?

## Skip Unless Needed

- Do not use this page for current standard symbol behavior.
- Do not add package management, broad formatting protocols, subprocess
  pipelines, streaming I/O, or source-level effect handlers through this page
  unless the task explicitly selects that proposal work.
