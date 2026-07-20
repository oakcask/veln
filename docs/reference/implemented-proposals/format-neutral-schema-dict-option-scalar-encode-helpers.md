# Format-Neutral Schema Dict Option Scalar Encode Helpers

Status: implemented

This record preserves the completed `Dict<String, Option<scalar>>`
format-neutral encode helper slice from
`schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields include
`Dict<String, Option<Int>>`, `Dict<String, Option<Bool>>`,
`Dict<String, Option<Float>>`, or `Dict<String, Option<String>>`.

The helper remains a validation/pass-through boundary over the supplied
schema-local visible record shape. It returns `Result<TRecord, String>` and
does not produce binary bytes.

At this historical slice, arbitrary recursive format-neutral encode eligibility
was not yet available.
Shapes such as `Dict<String, Dict<String, Int>>`,
`Option<Dict<String, Option<Int>>>`, and non-string dictionary keys remain
outside the generated encode helper surface. Dictionary-list encode helper
support is preserved in a sibling implemented proposal record.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-dict-option-scalar-encode/`
  checks successful direct helper calls and explicit schema encode expressions
  over a `Dict<String, Option<Int>>` field.
- `../../../examples/specification/check/format-neutral-schema-dict-scalar-encode-boundary/`
  checks that non-string dictionary keys remain outside the generated encode
  helper boundary while nested eligible values are accepted.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  generated helper resolution for all supported scalar option dictionary
  values plus the current recursive dictionary boundary.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
