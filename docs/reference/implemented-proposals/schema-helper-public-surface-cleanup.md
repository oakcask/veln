# Schema Helper Public Surface Cleanup

Status: implemented

This record closes the source-visible generated-helper cleanup slice from
[Schema Binary Pattern Boundary](../../proposals/schema-binary-pattern-boundary.md).
The current public source surface applies schemas with explicit operations:
`decode Schema from view at base_offset` for open-input decode and
`encode Schema from value` for schema-local encode.

## Current Routes

- Source syntax and path visibility:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Runtime lowering and compatibility helper boundary:
  [../../specification/execution.md](../../specification/execution.md).
- Source-visible helper inventory and effect routing:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Executable public examples:
  [../../../examples/specification/run/schema-decode-expression/](../../../examples/specification/run/schema-decode-expression/)
  and
  [../../../examples/specification/run/schema-encode-expression/](../../../examples/specification/run/schema-encode-expression/).

## Completion Evidence

Specification routes now describe explicit schema operations as the public way
to apply schemas. Generated schema helper names are compatibility and runtime
adapter details. Existing executable cases that still call those helper names
are retained only as compatibility or diagnostic migration evidence while the
compiler continues to accept them.

Ordinary Veln functions remain the boundary for projecting between
schema-local visible records and domain records.
