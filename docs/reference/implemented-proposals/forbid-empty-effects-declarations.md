# Forbid Empty Effects Declarations

Status: implemented

This record keeps the completed pure-declaration effect spelling change after
the behavior moved into the specification. It is historical evidence, not the
source for current behavior.

## Read First

- Current source declaration syntax:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current function, test, and function-type annotation rules:
  [../../specification/types.md](../../specification/types.md).
- Current effect labels, inference, and diagnostics:
  [../../specification/names-effects.md](../../specification/names-effects.md).

## Implemented Boundary

Pure function and test declarations omit the declaration-level
`effects [...]` clause. When a declaration-level clause is written, it must
contain at least one known effect label. The checker reports
`effect.empty_declaration` for declaration-level `effects []` and keeps repair
notes attached to that diagnostic.

Omission means the declared effect set is empty for public functions and tests.
If their bodies reach implemented effects such as `stdio`, `fs`, `process`, or
`concurrency`, the existing missing-effect diagnostics report the uncovered
effect and include call provenance. Private functions may still omit the
declaration-level clause and expose inferred direct or transitive effects to
callers.

Function type annotations are outside this boundary. A type such as
`fn(Int) -> Int effects []` still describes a pure callable value.

## Non-Goals Preserved

- Do not change the implemented effect labels.
- Do not require private effectful functions to declare non-empty effects.
- Do not change function type assignment compatibility.
- Do not ban empty effect lists in function type annotations.

## Completion Evidence

- Semantic tests cover empty declaration diagnostics for private functions,
  public functions, and tests.
- Semantic and CLI tests cover omitted pure public declarations, omitted
  effectful public declarations, omitted effectful tests, repair notes, and
  human output.
- Specification examples cover the public empty-declaration diagnostic through
  `examples/specification`.
- Source-surface fixtures and formatter-oriented tests use omission for pure
  declarations while preserving function-type `effects []`.

## Skip Unless Needed

- Do not read this page for current effect rules.
- Use this record only when auditing why `effects []` is invalid on
  declarations but still valid in function type annotations.
