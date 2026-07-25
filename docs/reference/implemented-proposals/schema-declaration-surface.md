# Schema Declaration Surface

Status: implemented

Schema composition uses the existing schema field type position. Local schema
paths, imported public paths, and schema aliases resolve independently from
ordinary types, and a composition binding exposes the target's visible record
only as one nested field. Format-neutral and binary schemas require matching
target formats.

Later length, repeat, dispatch, predicate, and validation expressions may use
explicit paths into earlier composed values. Decode and encode eligibility are
checked independently, composition cycles are rejected before typed IR, and
nested failures retain the containing schema, binding, target schema, and
nested field path without committing partial output.

Current behavior is specified in
[Source Surface](../../specification/source-surface.md) and
[Execution](../../specification/execution.md). Executable evidence is under
`examples/specification/run/schema-composition-binary-nested-paths/`,
`examples/specification/run/schema-composition-format-neutral/`, and
the matching format-neutral and binary `schema-composition-*-failure/` cases,
plus
`examples/specification/check/schema-composition-diagnostics/` and
`examples/specification/check/schema-composition-grammar-precedence/`. Parser
and formatter evidence remains in the syntax crate because composition adds no
grammar or keyword.

The boundary deliberately adds no schema-to-type alias, binary field family,
generated helper shape, or protocol-specific behavior.
