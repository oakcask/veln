# Format-Neutral Schema Nested Vec Scalar Encode Helpers

Status: implemented

This record preserves the completed nested anonymous-record `Vec<scalar>`
format-neutral encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`, `../../specification/execution.md`,
and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when an anonymous record field contains `Vec<scalar>` fields. The
supported scalar leaves are `Int`, `Bool`, `Float`, and `String`.

The helper preserves the schema-local visible record shape, returns
`Result<T, String>`, and passes the supplied record through on success without
producing binary bytes. Direct top-level `Vec<scalar>` fields continue to use
the same existing helper boundary.

This slice does not add arbitrary recursive `Vec<T>` encode eligibility.
Nested shapes such as `Vec<Vec<Int>>` remain outside the generated
format-neutral encode helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-nested-vec-scalar-encode/`
  checks direct helper calls and explicit schema encode expressions over an
  anonymous record field containing `Vec<Int>`, `Vec<Bool>`, `Vec<Float>`, and
  `Vec<String>` fields.
- `../../../examples/specification/check/format-neutral-schema-vec-scalar-encode-boundary/`
  keeps a nested recursive `Vec<Vec<Int>>` shape outside the generated encode
  helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution plus Core and IR lowering for nested anonymous-record
  `Vec<scalar>` fields.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
encode shapes, and later schema composition surfaces.
