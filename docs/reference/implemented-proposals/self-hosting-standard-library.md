---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Self-Hosting Standard Library

This page records the completed source-backed prelude helper migration. Use
the specification pages for current helper signatures and value semantics.
The later package migration superseded descriptor source metadata; see
[standard-library-package.md](standard-library-package.md) for that history.

## Read First

- Current prelude helper behavior and standard-package boundary:
  [../../specification/names-effects.md](../../specification/names-effects.md),
  then
  [../../specification/names-effects.md#standard-package-boundary](../../specification/names-effects.md#standard-package-boundary)
  when exact helper details matter.
- Current source syntax available to embedded library sources:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Use this page for completion evidence and cleanup routing only.

## Outcome

At the completion of this slice, the standard symbol table had no remaining
descriptor-only pure helpers. Compiler-known prelude helpers were represented
as source-backed descriptor entries with embedded standard-library metadata.

The later standard-package implementation replaced that source-metadata model.
Current project analysis gets declarations and bodies from `std::prelude`, uses
compiler adapters only for expected-type inference, and reserves intrinsics for
`prelude_builtin::*`. The JVM backend still provides those intrinsic runtime
operations.

This proposal record is now history and routing. New standard-library work
should use a new proposal page unless the behavior is already stated by
`../../specification/`.

## Completion Evidence

- `prelude` source contains the source-backed helper bodies and private
  support functions for reusable helper behavior.
- `vec_len` and `vec_fold` live in `prelude` and delegate to their
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
