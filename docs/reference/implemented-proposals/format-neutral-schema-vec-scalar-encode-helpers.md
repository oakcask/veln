# Format-Neutral Schema Vec Scalar Encode Helpers

Status: implemented

This record preserves the completed `Vec<scalar>` format-neutral encode
helper slice from `schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is a scalar leaf,
`Option<scalar>` field, `Option<List<scalar>>` field, `List<scalar>` field,
`Vec<scalar>` field, `Vec<Option<scalar>>` field,
`Dict<String, scalar>` field, `Result<scalar, scalar>` field, or anonymous
record field whose fields are supported format-neutral encode shapes. The
scalar leaves are `Int`, `Bool`, `Float`, and `String`.

The helper accepts the same schema-local visible record shape as the generated
format-neutral decode helper and returns `Result<T, String>`, preserving the
supplied record on success without producing binary bytes. Explicit
`encode Schema from value` expressions use the same helper boundary.

At this historical slice, arbitrary recursive `Vec<T>` encode eligibility was
not yet available. The
later `Vec<Option<scalar>>` encode slice is recorded in
[Format-Neutral Schema Option Vec Encode Helpers](format-neutral-schema-option-vec-encode-helpers.md).

## Evidence

- `../../../examples/specification/run/format-neutral-schema-vec-scalar-encode/`
  checks direct helper calls and explicit schema encode expressions over
  `Vec<scalar>` fields.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation plus Core and IR lowering.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary helper
families outside the implemented slices are separate proposals or explicit
non-goals.
