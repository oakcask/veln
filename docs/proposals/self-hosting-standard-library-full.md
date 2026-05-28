# Self-Hosting Standard Library Full

Status: proposed

This file keeps source-backed standard library proposal details after
implemented standard library behavior moved to the language specification.

## Goal

Move ordinary reusable behavior into Veln libraries while keeping the compiler
responsible only for primitive runtime boundaries and compatibility metadata
that cannot yet be expressed in source.

## Promoted Source-Backed Helper Target

The selected `option_map` target has moved from a descriptor-only prelude
helper into the source-backed pure-helper model already used by
`option_unwrap_or`. Current behavior now belongs in
`../specification/names-effects.md`.

The promotion stayed inside the existing helper path:

- keep the source-visible `option_map(value, f)` signature compatible with the
  current prelude adapter
- add ordinary Veln source for the helper beside other core prelude source
- record source metadata on the standard symbol descriptor
- keep effects empty and keep diagnostics anchored on user call sites
- keep backend lowering and public helper semantics compatible with the
  implemented behavior

## Later Library Layers

Later source-backed library work may cover:

- collection helpers beyond the implemented compiler-known surface
- string helpers beyond primitive runtime support
- file-system helpers above the minimal descriptor-backed boundary
- process helpers above the minimal descriptor-backed boundary
- compiler-support helpers that can be written in ordinary Veln source

## Boundary

The current specification defines the implemented effect labels and
compiler-known calls. Future self-hosting work should decide how source-level
effect abstractions replace or wrap those built-in labels.

Future targets must not add package management, source-level effect handlers,
broad module loading, subprocess pipelines, streaming I/O, or new public
complexity guarantees through this proposal.

## Open Questions

- Which descriptor-only pure helper should follow `option_map` after the
  source-backed pattern is validated?
- Which helpers must remain runtime intrinsics until user-defined effects or
  effect handlers exist?
- Should process termination keep the current return shape or use a future
  never type?
- How should library modules expose host paths without treating paths as plain
  strings at type boundaries?

## Promotion Rule

When future self-hosting behavior becomes implemented, document it in
`../specification/` first and leave only absent behavior here.
