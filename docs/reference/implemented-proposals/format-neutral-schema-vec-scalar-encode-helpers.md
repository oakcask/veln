# Format-Neutral Schema Vec Scalar Encode Helpers

Status: implemented

This record preserves the completed `Vec<scalar>` format-neutral encode
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is a scalar leaf,
`Option<scalar>` field, `Option<List<scalar>>` field, `List<scalar>` field,
`Vec<scalar>` field, `Dict<String, scalar>` field,
`Result<scalar, scalar>` field, or anonymous record field whose fields are
supported format-neutral encode shapes. The scalar leaves are `Int`, `Bool`,
`Float`, and `String`.

The helper accepts the same schema-local visible record shape as the generated
format-neutral decode helper and returns `Result<T, String>`, preserving the
supplied record on success without producing binary bytes. Explicit
`encode Schema from value` expressions use the same helper boundary.

This slice does not add arbitrary recursive `Vec<T>` encode eligibility.
Shapes such as `Vec<Option<Int>>` remain outside the generated encode helper
boundary.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-vec-scalar-encode/`
  checks direct helper calls and explicit schema encode expressions over
  `Vec<scalar>` fields.
- `../../../examples/specification/check/format-neutral-schema-vec-scalar-encode-boundary/`
  checks that `Vec<Option<Int>>` remains outside the generated encode helper
  boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation plus Core and IR lowering.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
encode shapes, source ADT encode fields, and later schema composition
surfaces.
