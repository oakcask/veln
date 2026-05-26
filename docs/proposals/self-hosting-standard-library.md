# Self-Hosting Standard Library

Status: promoted
Implementation: implemented subset: descriptor-backed standard symbols,
minimal `fs` and `process` intrinsics, one source-backed pure helper, and the
compiler-support source-loading trial are promoted to the language reference.

This is the routing page for the promoted standard library and compiler-known
intrinsic work needed for eventual self-hosting. Use current behavior under
`../reference/language/` first, and use the full proposal only for historical
implementation context.

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
  table incrementally. The implemented subset routes stdio effects,
  concurrency effects, and prelude helper admission through the descriptor
  table while keeping existing type adapters and runtime lowering paths.
- Add minimal `fs` and `process` intrinsics with coarse effects. The
  implemented subset includes descriptor-backed signatures, effect inference,
  public-boundary diagnostics, checked-core and IR lowering, and JVM runtime
  operations for the accepted minimal surface.
- Move pure helpers behind Veln source implementations when the language can
  express them. The implemented subset embeds a Veln source implementation for
  one pure helper while keeping the existing descriptor-backed type adapter and
  runtime lowering path.
- Exercise one small compiler subsystem through the standard library subset.
  The implemented subset embeds and tests a source-loading compiler support
  helper that reads source text through descriptor-backed `fs`.

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
