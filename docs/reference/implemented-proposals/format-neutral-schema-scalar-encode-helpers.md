# Format-Neutral Schema Scalar Encode Helpers

Status: implemented

This record preserves the completed scalar-only format-neutral encode helper
slice from `../../proposals/schema-declaration-surface.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is a scalar leaf: `Int`,
`Bool`, `Float`, or `String`. The helper accepts the same schema-local visible
record shape as the generated format-neutral decode helper and returns
`Result<T, String>`, preserving the supplied record on success without
producing binary bytes.

Explicit `encode Schema from value` expressions use the same scalar-only
format-neutral helper boundary. At this slice, unsupported format-neutral
encode helper shapes, including container fields, did not expose the encode
helper. Later format-neutral encode helper slices are tracked in sibling
implemented proposal records.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-scalar-encode/case.toml`
  checks direct helper calls and explicit schema encode expressions over a
  scalar-only format-neutral schema.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation, Core and IR lowering, and the superseding
  recursive visible-shape eligibility rule.

## Superseding Work

The recursive format-neutral encode boundary is complete and recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for binary schema
fields outside the implemented helper slices and later schema composition
surfaces.
