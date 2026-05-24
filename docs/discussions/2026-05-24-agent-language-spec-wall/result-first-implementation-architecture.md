# Discussion Result: First Implementation Architecture

Date: 2026-05-24

## Picked Question

- Which host implementation languages and internal toolchain split should the
  first Veln implementation use while keeping typed IR runtime-neutral?

## Decision

Implement the first Veln toolchain with a Rust CLI, parser, formatter, checker,
semantic analysis pipeline, checked-core lowering, typed IR representation, and
initial JVM backend driver.

Keep the typed IR as the main internal boundary between target-independent
language analysis and target-specific execution. The first JVM path should be
implemented from the Rust toolchain at first, either by generating Java source
or by emitting JVM class files later. The JVM runtime support library may be
implemented in Java or Kotlin and should provide host-managed representations
and helpers for Veln values, prelude functions, contract failures, stdio
events, source spans, and entry dispatch.

Do not split the first JVM backend into a separate Kotlin compiler component
before the `check`, `fmt`, `run`, and `test` loop is working. Kotlin remains a
good later implementation language for a richer JVM backend, especially if the
project moves toward bytecode generation, JVM library integration, or more
target-specific optimization. That split should happen after the typed IR has
enough examples to show which backend API is stable enough to maintain.

The first implementation sequence is:

1. Rust CLI, parser, formatter, checker, semantic tables, checked core, and
   typed IR.
2. Rust-owned JVM lowering, initially allowed to generate Java source.
3. Small Java or Kotlin JVM runtime library for Veln values and runtime
   services.
4. Optional later JVM backend refactor to class-file generation or a Kotlin
   backend module.
5. Node-hosted WebAssembly experiments only after the reference JVM path proves
   the typed IR boundary.

## Rationale

Rust fits the first target-independent implementation because the compiler
needs deterministic data structures, arena-backed source nodes, stable
diagnostic snapshots, source-span-heavy tests, and straightforward single-file
binary distribution. It also fits the existing decision to keep source AST nodes
and phase-specific side tables separate.

The main architectural risk is not which host language lowers to the JVM; it is
letting JVM details leak into the typed IR. Keeping the first backend in Rust
avoids an early cross-language compiler API, schema versioning problem, and
multi-build-system workflow while the IR shape is still changing. The runtime
library is a better place to use Java or Kotlin early because it naturally lives
on the JVM and can expose ordinary host-managed helpers to generated code.

Generating Java source is acceptable for the first slice if it gets execution
working quickly and keeps source spans, contract failures, stdio events, and
test capture easy to inspect. Class-file emission or a Kotlin backend can be
introduced later as an implementation optimization, not as a first-slice
language requirement.

This preserves the earlier runtime-target decision: `check` and `fmt` remain
target-independent, runnable code lowers through typed IR, the JVM is the first
reference execution target, and Node-hosted WebAssembly stays experimental.

## First-Slice Rules

- The Rust toolchain owns source parsing, formatting, semantic analysis,
  diagnostics, checked core, typed IR construction, and the first JVM lowering
  path.
- Typed IR must describe Veln semantics, not JVM implementation details. It
  should not expose JVM class names, descriptors, stack locals, object identity,
  or runtime-library layout as language facts.
- The JVM runtime library may be Java or Kotlin, but its public helper surface
  is an implementation detail during the first slice.
- Prelude signatures and static semantics belong to the target-independent
  frontend. Runtime helper implementations belong to the selected target
  runtime.
- The first JVM backend may generate Java source before it emits class files.
  Generated Java source is a backend artifact, not a language specification.
- A separate Kotlin JVM backend module should wait until the typed IR and
  reference examples are stable enough to justify a maintained cross-language
  backend API.
- Node-hosted WebAssembly should be used later as a pressure test for typed IR
  neutrality, not as a co-equal first implementation path.

## Open Details

The exact Rust crates, parser library, arena library, JVM bytecode library,
Java-versus-Kotlin runtime choice, generated-source layout, and build-system
integration remain open implementation details.

The typed IR serialization format is also open. The first implementation may
keep the IR as Rust data structures with debug or snapshot JSON for tests,
without promising a stable external IR schema.

## Consequence

The first slice can start implementation with one primary compiler language,
one reference runtime target, and one small JVM runtime library. This keeps the
toolchain simple enough to deliver `check`, `fmt`, `run`, and `test` while
preserving the architectural boundary that matters most: Veln typed IR remains
runtime-neutral, and JVM details stay in the backend and runtime library.
