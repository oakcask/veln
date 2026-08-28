---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Path Runtime Representation

This record keeps the completed path-representation follow-up after the
source-visible `Path` and `String` assignment boundary was already implemented.
It is historical evidence, not the source for current behavior.

## Read First

- Current type boundary:
  [../../specification/names-effects.md](../../specification/names-effects.md)
  and [file system calls](../../specification/names-effects.md#file-system-calls)
  plus [process calls](../../specification/names-effects.md#process-calls).
- Current value and runtime semantics:
  [../../specification/execution.md](../../specification/execution.md) and
  [../../specification/execution-full.md](../../specification/execution-full.md).
- Current assignment compatibility:
  [../../specification/types.md](../../specification/types.md).

## Implemented Boundary

`Path` is source-visible at implemented `fs` and `process` boundaries, and it
is not assignment-compatible with `String`. Runtime path values are
backend-owned values. That representation detail does not authorize source
assignment compatibility or a public path layout guarantee.

The source-visible boundary keeps `Path` on implemented `fs` parameters and the
`process::cwd` result, removes the special `Path` and `String` assignment
bridge, and documents that the types are distinct at assignment boundaries.
Semantic coverage rejects `String` at every implemented `fs` path parameter and
rejects `process::cwd()` where `Result<String, ProcessError>` is expected.

## Completed Target

The runtime representation preserves an owned host path value across
implemented standard-library calls. The observable requirement is that `Path`
values returned by `process::cwd` and `fs::read_dir` remain usable by later
`fs` calls without being represented as source-visible `String` values.

## Non-Goals Preserved

- Do not weaken the current `Path` and `String` assignment boundary.
- Do not expose a public path layout, encoding, or normalization guarantee.
- Do not change `fs` or `process` signatures as part of this representation
  migration.
- Do not make path behavior depend on repository-local absolute paths.

## Completion Evidence

- Semantic tests reject `String` where `Path` is required and reject `Path`
  where `String` is required by assignment compatibility.
- JVM runtime tests pass `Path` values from `process::cwd` and `fs::read_dir`
  through public standard-library calls rather than inspecting internal
  storage.
- Toolchain specification cases continue to cover the public `fs` and
  `process` behavior through `examples/specification`.
