# Self-Hosting Standard Library Full

Status: proposed

This file keeps future self-hosting questions after implemented standard
library behavior moved to the language specification.

## Goal

Move ordinary reusable behavior into Veln libraries while keeping the compiler
responsible only for primitive runtime boundaries that cannot yet be expressed
in source.

## Future Library Layers

Future source-backed library work may cover:

- collection helpers beyond the implemented compiler-known surface
- string helpers beyond primitive runtime support
- file-system helpers above the minimal descriptor-backed boundary
- process helpers above the minimal descriptor-backed boundary
- compiler-support helpers that can be written in ordinary Veln source

## Effect Boundary

The current specification defines the implemented effect labels and compiler-known
calls. Future self-hosting work should decide how source-level effect
abstractions replace or wrap those built-in labels.

## Open Questions

- What is the boundary between source-backed helper code and compiler-known
  descriptor metadata?
- Which helpers must remain runtime intrinsics until user-defined effects or
  effect handlers exist?
- Should process termination keep the current return shape or use a future
  never type?
- How should library modules expose host paths without treating paths as plain
  strings at type boundaries?

## Promotion Rule

When future self-hosting behavior becomes implemented, document it in
`../specification/` first and leave only absent behavior here.
