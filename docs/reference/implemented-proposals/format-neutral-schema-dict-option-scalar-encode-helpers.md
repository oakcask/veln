# Format-Neutral Schema Dict Option Scalar Encode Helpers

Status: implemented

This record preserves the completed `Dict<String, Option<scalar>>`
format-neutral encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
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

This slice does not add arbitrary recursive format-neutral encode eligibility.
Shapes such as `Dict<String, List<Int>>`,
`Dict<String, Dict<String, Int>>`, `Option<Dict<String, Option<Int>>>`, and
non-string dictionary keys remain outside the generated encode helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-dict-option-scalar-encode/`
  checks successful direct helper calls and explicit schema encode expressions
  over a `Dict<String, Option<Int>>` field.
- `../../../examples/specification/check/format-neutral-schema-dict-scalar-encode-boundary/`
  checks that non-string dictionary keys and unsupported dictionary value
  shapes remain outside the generated encode helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  generated helper resolution for all supported scalar option dictionary
  values plus adjacent rejected dictionary boundaries.

## Remaining Work

The broader schema declaration proposal remains open for format-neutral encode
helpers beyond the implemented scalar, supported container,
dictionary-option, option-dictionary, scalar-result, result-option, anonymous
record, and source ADT shapes, binary schema fields outside the implemented
helper slices, and later schema composition surfaces.
