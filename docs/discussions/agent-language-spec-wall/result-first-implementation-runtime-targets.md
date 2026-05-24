# Discussion Result: First Implementation Runtime Targets

## Picked Question

- Which runtime target strategy should the first Veln implementation use if
  implementing a custom GC or VM would expand the prototype scope too much?

## Decision

Use a backend-neutral typed IR as the implementation boundary, with the JVM as
the first reference execution target and Node-hosted WebAssembly as an
experimental secondary target.

`veln check` and `veln fmt` should remain target-independent. They operate on
the source AST, type tables, contract analysis, effect analysis, and diagnostic
data. `veln run` and `veln test` should execute only after the selected entry
point has passed static gates and has been lowered to a typed executable IR.

The first JVM backend should compile or generate code that uses host-managed
values and the JVM's garbage collector. Veln values such as `String`, records,
lists, dictionaries, `Option`, `Result`, contract failures, and stdio events
may be represented by a small Veln runtime library on the JVM. Public Veln
functions can initially lower to ordinary static entry points rather than a
larger object model.

The first Node/WASM backend should be treated as an embedding experiment, not
as the reference runtime. It may use JavaScript glue code for host-managed
strings, aggregate values, stdio capture, contract error records, and entry
dispatch. Avoid designing a Veln-owned linear-memory heap, collector, object
layout, or full ABI until examples show that pure WASM execution is necessary.

This decision does not make the JVM or WASM object layout part of the Veln
language specification. Backend ABIs, class names, exported WASM functions,
and glue-code shapes are implementation details during the first slice.

## Rationale

The first-slice mutability decision already specifies automatic memory
management while leaving the concrete GC strategy unspecified. Targeting
mature host runtimes preserves that language contract without forcing the
prototype to implement a collector, scheduler, object allocator, finalization
model, or VM instruction set.

The JVM is a good reference target for the first slice because its managed
object model maps directly onto Veln's current value surface: strings,
immutable aggregates, result-like values, source-linked runtime errors, and a
small prelude. It also keeps integration with existing Java systems practical,
which matters for adoption and embedding experiments.

Node-hosted WebAssembly is still useful, but it should not carry the whole
language runtime at first. Plain WASM's linear memory model makes rich strings,
records, persistent containers, and runtime error payloads expensive unless
the implementation either builds its own heap discipline or leans on host glue.
For the prototype, leaning on JavaScript glue is the smaller and more
reversible choice.

This split keeps the compiler architecture honest. The language front end
cannot bake in JVM-only assumptions, but the project also avoids treating every
backend as equally production-ready before `check`, `run`, `test`, and the
diagnostic loop are proven.

## First-Slice Rules

- The compiler should lower checked, hole-free runnable code into a typed IR
  before target-specific execution.
- `check` and `fmt` must not require a selected runtime target.
- The JVM backend is the first reference execution target for `run` and
  `test`.
- The Node/WASM backend is experimental in the first slice and may require
  JavaScript glue code.
- The first implementation may include small runtime libraries for host value
  representation, prelude helpers, contract failures, stdio events, source
  spans, and entry dispatch.
- The first implementation should not expose JVM classes, WASM exports,
  JavaScript glue objects, object identity, pointer identity, or allocation
  behavior as source-language semantics.
- The first implementation should not commit to a stable cross-backend ABI
  until the runnable IR and reference backend have enough examples.
- A backend may report unsupported-target diagnostics when a checked program
  uses a feature that has not yet been lowered for that target.

## Open Details

The exact IR shape remains open. It should be small enough to drive runtime
code generation, source span preservation, contract insertion, effectful stdio
handling, and test event capture without becoming a second source language.

The exact JVM code generation route is not fixed. Generating Java source,
emitting class files, using an existing bytecode library, or interpreting the
typed IR on the JVM are all compatible with this decision if the reference
backend remains host-managed.

The exact Node/WASM host boundary is not fixed. Early experiments may pass
opaque handles through WASM, use imported host functions, or compile only pure
subgraphs to WASM while keeping aggregate-heavy behavior in JavaScript glue.

WASI support is not a security boundary for the first Veln implementation. If
later versions need sandboxing, that should be specified as a separate runtime
and threat-model decision.

## References

- Node.js documentation, "Node.js with WebAssembly":
  https://nodejs.org/uk/learn/getting-started/nodejs-with-webassembly
- Node.js documentation, "WebAssembly System Interface (WASI)":
  https://nodejs.org/api/wasi.html
- WebAssembly specifications:
  https://webassembly.org/specs/
- Java Virtual Machine Specification:
  https://docs.oracle.com/en/java/javase/26/docs/specs/jvms26.pdf

## Consequence

The first Veln implementation can move toward executable programs without
building a custom GC or VM. The front end and diagnostics stay target-neutral,
the JVM gives the project one mature reference runtime, and Node/WASM remains
available for embedding experiments without forcing an early custom heap or
stable ABI.
