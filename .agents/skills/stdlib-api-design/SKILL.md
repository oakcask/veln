---
name: stdlib-api-design
description: Design, change, or review public Veln standard-library APIs, including exports, prelude names, and compiler-known host calls. Excludes private implementation changes that preserve the public contract.
---

# Standard-Library API Design

Read the relevant current specification through the Veln repository route.
Apply [standard-library-api-design.md](../../../docs/reference/standard-library-api-design.md),
which owns the design requirements. Use the ADT naming and test placement
policies it links when those concerns are affected.

Inspect package exports, source declarations, compiler-known descriptors, and
host mappings to identify what consumers can actually call. Do not infer
public exposure from an internal `pub` declaration alone or assume removing
a source wrapper removes a compiler-known entry point.

Review a representative call and failure/recovery path. Compare related
operations for cardinality, control arguments, effects, typed payloads,
authority, committed progress, and canonical ownership. Keep semantic reasons
for specialization; do not propose changes only to make names symmetrical.

Check generic-function support separately from generic ADTs, compiler-special
helpers, and effect-row polymorphism. Preserve a forwarding callback's result
and effects when supported; otherwise name the implementation dependency in
the focused proposal rather than pretending it already works.

For a changed contract, identify independent expected outcomes and meaningful
failure/state-preservation checks. Use companion tests for stdlib behavior and
separate toolchain cases only for identified checker, backend, or host contracts.
Apply effect-handler maintenance to handler, host-boundary, or scoped-fake work.

Use proposal-implementation-audit for proposal lifecycle and symbol-rename-audit
for exported renames or removals. Update the current specification only when
implementation and evidence support it. Report material limitations and
unresolved design decisions without claiming planned tests have passed.
