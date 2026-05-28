# Self-Hosting Standard Library Full

Status: proposed

This file keeps source-backed standard library proposal details after
implemented standard library behavior moved to the language specification. Read
[self-hosting-standard-library.md](self-hosting-standard-library.md) first.

## Goal

Move ordinary reusable behavior into Veln libraries while keeping the compiler
responsible only for primitive runtime boundaries and compatibility metadata
that cannot yet be expressed in source.

## Migration Pattern

The source-backed pure-helper pattern already exists for helpers documented in
`../specification/names-effects.md`. That specification page, not this
proposal, defines implemented helper signatures, value semantics, source
metadata, diagnostics, descriptor table behavior, and the current
source-backed versus descriptor-only split.

Future work should preserve the implemented pattern without restating helper
semantics here:

- choose a helper whose signature and value semantics are already implemented
- add ordinary Veln source beside other core prelude source
- record source metadata on the standard symbol descriptor
- keep effects empty for pure helpers
- keep diagnostics anchored on user call sites
- keep backend lowering and public helper semantics compatible with the
  implemented behavior

## Remaining Pure Helper Candidates

Remaining source-backed prelude work should choose from helpers that satisfy
the migration pattern above. The proposal changes only where reusable helper
bodies can live and what descriptor metadata records about that source.

Prefer a candidate when:

- it is listed as a descriptor-only pure prelude helper in
  `../specification/names-effects.md`
- its signature and value semantics are already specified there
- its behavior is expressible in existing Veln source
- it needs no new effect label, runtime boundary, parser feature, or public
  complexity promise
- it can continue using the descriptor-backed signature adapter and backend
  lowering during migration

Avoid a candidate when it depends on host I/O, process state, broad module
loading, source-level effect handlers, streaming, subprocesses, or a container
representation guarantee.

## Later Layers

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

- Which descriptor-only pure helper should move next after the source-backed
  option and result helper pattern?
- Which helpers must remain runtime intrinsics until user-defined effects or
  effect handlers exist?
- Should process termination keep the current return shape or use a future
  never type?
- How should library modules expose host paths without treating paths as plain
  strings at type boundaries?

## Promotion Rule

When future self-hosting behavior becomes implemented, document it in
`../specification/` first and leave only absent behavior here.
