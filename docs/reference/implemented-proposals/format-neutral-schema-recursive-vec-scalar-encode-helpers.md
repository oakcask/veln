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
`Result<T, String>` without producing binary bytes. At this historical slice,
this was a bounded shape rather than a general recursive-container rule.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-recursive-vec-scalar-encode/`
  checks all four scalar leaves, direct helper calls, explicit encode
  expressions, and nested anonymous-record placement.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution plus Core and IR lowering across deeper recursive vector
  shapes.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
