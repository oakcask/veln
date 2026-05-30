# Self-Hosting Standard Library Full

Status: proposed

This file keeps the reusable candidate and migration rule for source-backed
standard library work. Read
[self-hosting-standard-library.md](self-hosting-standard-library.md) first; open
this file only after the specification identifies one descriptor-only helper as
the target.

## Goal

Move ordinary reusable behavior into Veln libraries while keeping the compiler
responsible only for primitive runtime boundaries and compatibility metadata
that cannot yet be expressed in source.

## Candidate And Migration Rule

Use this rule only after the short page routes the work to exactly one
descriptor-only pure helper from the implemented specification. Prefer a
candidate when the specification already provides its signature, value
semantics, and descriptor-only status, and when:

- its behavior is expressible in existing Veln source
- it needs no new effect label, runtime boundary, parser feature, or public
  container representation guarantee
- it can continue using the descriptor-backed signature adapter and backend
  lowering during migration

Avoid a candidate when it depends on host I/O, process state, broad module
loading, source-level effect handlers, streaming, subprocesses, or a container
representation guarantee.

Keep helper semantics in the specification and apply only this source-backed
placement pattern:

- choose a helper whose signature and value semantics are already implemented
- add ordinary Veln source beside other core prelude source
- record source metadata on the standard symbol descriptor
- keep effects empty for pure helpers
- keep diagnostics anchored on user call sites
- keep backend lowering and public helper semantics compatible with the
  implemented behavior

## Remaining Pure Helper Candidates

Remaining source-backed prelude work chooses from the descriptor-only pure
helpers listed in
[../specification/names-effects.md](../specification/names-effects.md).

For signatures, value behavior, and the source-backed versus descriptor-only
split, return to the specification. This proposal only keeps the candidate
selection rule, migration pattern, and future-work boundary for behavior that
is not yet source backed.

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

- Which descriptor-only pure helper should become the next source-backed
  target?
- Which helpers must remain runtime intrinsics until user-defined effects or
  effect handlers exist?
- Should process termination keep the current return shape or use a future
  never type?
- How should library modules expose host paths without treating paths as plain
  strings at type boundaries?

## Promotion Rule

When future self-hosting behavior becomes implemented, document it in
`../specification/` first and leave only absent behavior here.
