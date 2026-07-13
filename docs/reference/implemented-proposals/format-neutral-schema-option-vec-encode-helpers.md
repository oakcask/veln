# Format-Neutral Schema Option Vec Encode Helpers

Status: implemented

This record preserves the completed `Vec<Option<scalar>>` format-neutral
encode helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is one of the supported
format-neutral encode shapes, including `Vec<Option<scalar>>` fields. The
supported scalar leaves are `Int`, `Bool`, `Float`, and `String`.

The helper accepts the schema-local visible record shape, returns
`Result<T, String>`, and preserves the supplied record on success without
producing binary bytes. Explicit `encode Schema from value` expressions use
the same helper boundary.

At this historical slice, recursive `Vec<T>` encode eligibility was not yet
available. The later
bounded `Vec<Vec<scalar>>` slice is recorded in
[Format-Neutral Schema Recursive Vec Scalar Encode Helpers](format-neutral-schema-recursive-vec-scalar-encode-helpers.md).

## Evidence

- `../../../examples/specification/run/format-neutral-schema-option-vec-encode/`
  checks direct helper calls and explicit schema encode expressions over
  `Vec<Option<scalar>>` fields.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation plus Core and IR lowering across the recursive
  visible-shape boundary.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
