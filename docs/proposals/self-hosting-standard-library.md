# Self-Hosting Standard Library

Status: accepted-proposal
Implementation: not implemented

This is the routing page for standard library and compiler-known intrinsic work
needed for eventual self-hosting. Use the full proposal only after selecting
this target from [target-queue.md](target-queue.md).

## Read First

- Current names, prelude, stdio, and effect behavior:
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- Promotion boundary:
  [../document-status.md](../document-status.md).
- Full implementation proposal:
  [self-hosting-standard-library-full.md](self-hosting-standard-library-full.md).

## Target

Move ordinary reusable behavior into Veln libraries while keeping the compiler
responsible for only the primitive runtime boundary it must know before
user-defined effects and effect handlers exist.

The accepted implementation path is:

- Add a descriptor table for compiler-known standard symbols.
- Route existing `stdio`, concurrency, and prelude helper metadata through that
  table incrementally.
- Add minimal `fs` and `process` intrinsics with coarse effects.
- Move pure helpers behind Veln source implementations when the language can
  express them.
- Exercise one small compiler subsystem through the standard library subset.

## Use When

- Adding `fs` or `process` effect labels.
- Adding standard library symbols needed by a self-hosted compiler.
- Moving a compiler-known helper toward a descriptor-backed or Veln-backed
  implementation.
- Deciding whether a new helper belongs in library source, runtime intrinsic
  metadata, or ordinary compiler code.

## Skip Unless Needed

- Do not treat this proposal as current behavior unless
  `../reference/language/` also states the implemented subset.
- Do not open the full proposal when current behavior pages already answer the
  question.
- Do not add package management, broad formatting protocols, subprocess
  pipelines, streaming I/O, or source-level effect handlers through this
  target.
