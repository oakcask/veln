# Implementation Boundary Decisions

Read these records only when runtime, AST, architecture, mutability, or
compatibility boundaries need rationale.

## Read First

- Current execution boundary: [../../specification/execution.md](../../specification/execution.md).
- Current name, prelude, stdio, and effect behavior:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Current source and metadata behavior:
  [../../specification/source-surface.md](../../specification/source-surface.md).

## Read When

- Use the sections below only after the implemented reference identifies an
  implementation boundary without enough rationale for the task.
- Open an individual `result-*.md` record only for the selected architecture,
  runtime, or compatibility topic.

## Architecture And AST

- [AST Implementation Representation](records/result-ast-implementation-representation.md)
- [AST Phase Boundary](records/result-ast-phase-boundary.md)
- [First Implementation Architecture](records/result-first-implementation-architecture.md)
- [First Implementation Runtime Targets](records/result-first-implementation-runtime-targets.md)
- [Module Metadata Location](records/result-module-metadata-location.md)

## Runtime Boundaries

- [Channel-First Concurrency Runtime](records/result-channel-first-concurrency-runtime.md)
- [Contract Blame Boundary](records/result-contract-blame-boundary.md)
- [Hole Runtime Boundary](records/result-hole-runtime-boundary.md)
- [Prelude Complexity Guarantees](records/result-prelude-complexity-guarantees.md)
- [Runtime Value Freeze Boundary](records/result-runtime-value-freeze-boundary.md)
- [Transitive Effect Diagnostics](records/result-transitive-effect-diagnostics.md)

## Toolchain Dependencies

- [Internal SHA-256 Backend](records/result-internal-sha256-backend.md)

## Skip Unless Needed

Use [../../specification/execution.md](../../specification/execution.md),
[../../specification/names-effects.md](../../specification/names-effects.md), or the
task-specific language page before opening these decision records for
implemented behavior.
