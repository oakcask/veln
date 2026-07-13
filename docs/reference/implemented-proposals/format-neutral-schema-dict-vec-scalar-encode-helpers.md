# Format-Neutral Schema Dict Vec Scalar Encode Helpers

Status: implemented

This record preserves the completed `Dict<String, Vec<scalar>>`
format-neutral encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields include
`Dict<String, Vec<Int>>`, `Dict<String, Vec<Bool>>`,
`Dict<String, Vec<Float>>`, or `Dict<String, Vec<String>>`.

The helper accepts the schema-local visible record shape, returns
`Result<T, String>`, and preserves the supplied record on success without
producing binary bytes.

At this historical slice, arbitrary recursive dictionary or vector encode
eligibility was not yet available. A later slice accepted
`Dict<String, Vec<Option<scalar>>>`. Non-string dictionary keys remain outside
the generated encode helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-dict-vec-scalar-encode/`
  checks successful direct helper calls and explicit schema encode expressions
  over `Dict<String, Vec<scalar>>` fields.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  generated helper resolution for all supported scalar vector dictionary
  values plus adjacent dictionary-vector coverage.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
