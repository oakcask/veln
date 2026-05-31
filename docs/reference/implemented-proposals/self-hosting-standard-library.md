# Self-Hosting Standard Library

Status: implemented

This page records the completed source-backed prelude helper migration. Use
the specification pages for current helper signatures, value semantics,
descriptor metadata, and source-backed status.

## Read First

- Current prelude helper behavior and source-backed boundary:
  [../../specification/names-effects.md](../../specification/names-effects.md),
  then
  [../../specification/names-effects-full.md#prelude-helpers](../../specification/names-effects-full.md#prelude-helpers)
  when exact helper details matter.
- Current source syntax available to embedded library sources:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Use this page for completion evidence and cleanup routing only.

## Outcome

The implemented standard symbol table has no remaining descriptor-only pure
helpers. All compiler-known prelude helpers listed by the current
specification are source-backed descriptor entries with embedded standard
library metadata.

The checker still uses descriptor-backed helper signatures, and the JVM backend
still lowers the helpers through the existing runtime operations. This keeps
public helper behavior, effect inference, and diagnostics anchored on user call
sites while embedded Veln source is checked as the source-backed body for each
helper entry point.

This proposal record is now history and routing. New source-backed standard
library work should use a new proposal page unless the behavior is already
stated by `../../specification/`.

## Completion Evidence

- `core_prelude` source contains the source-backed helper bodies and private
  support functions for reusable helper behavior.
- `vec_len` and `vec_fold` live in `core_prelude` and delegate to their
  matching `prelude_builtin` operations, while traversal-oriented vec helpers
  call `prelude_builtin::vec_fold` explicitly.
- Standard symbol descriptor tests verify that source-backed descriptors carry
  source metadata, private support functions are not public prelude
  descriptors, and no descriptor-only pure helpers remain.
- Semantic tests check that embedded source is checkable and that helper
  diagnostics stay anchored on user call sites.

## Boundary

- Float operator compatibility descriptors remain outside the source-backed
  pure-helper candidate pool.
- Host I/O, process state, broad module loading, source-level effect handlers,
  streaming, subprocess behavior, and public container representation
  guarantees were not added by this migration.
- Future library behavior beyond the implemented compiler-known prelude helper
  surface needs its own proposal route before implementation.

## Read When

- Checking why the self-hosting standard-library proposal target is no longer
  listed as active.
- Reviewing completion evidence before removing or superseding this record.
- Auditing source-backed helper migration boundaries.

## Skip Unless Needed

- Do not read this page for ordinary current prelude helper behavior.
- Do not use this page as a source of current helper signatures or value
  semantics.
