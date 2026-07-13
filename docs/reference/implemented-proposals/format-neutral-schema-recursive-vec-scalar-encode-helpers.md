# Format-Neutral Schema Recursive Vec Scalar Encode Helpers

Status: implemented

This record preserves the completed bounded `Vec<Vec<scalar>>`
format-neutral encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when a direct field or supported anonymous record field has the
exact shape `Vec<Vec<T>>`, where `T` is `Int`, `Bool`, `Float`, or `String`.

The helper preserves the schema-local visible record shape and returns
`Result<T, String>` without producing binary bytes. This is a bounded shape,
not a general recursive-container rule; `Vec<Vec<Vec<T>>>` and adjacent mixed
container shapes remain outside this slice.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-recursive-vec-scalar-encode/`
  checks all four scalar leaves, direct helper calls, explicit encode
  expressions, and nested anonymous-record placement.
- `../../../examples/specification/check/format-neutral-schema-vec-scalar-encode-boundary/`
  keeps `Vec<Vec<Vec<Int>>>` outside the generated encode helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution, Core and IR lowering, and the deeper negative boundary.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, adjacent or deeper format-neutral
encode shapes, and later schema composition surfaces.
