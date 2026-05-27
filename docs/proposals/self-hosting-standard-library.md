# Self-Hosting Standard Library

Status: open-proposal

This page tracks future self-hosting standard library questions whose behavior
is absent from the current reference. Implemented standard symbols, effects,
and compiler-known calls live in
[../reference/language/names-effects.md](../reference/language/names-effects.md).

## Read First

- Current names, prelude, stdio, file-system, process, and effect behavior:
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- Current source surface:
  [../reference/language/source-surface.md](../reference/language/source-surface.md).
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
  unless the target queue accepts that work.
