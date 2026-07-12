# Format-Neutral Schema Dict Vec Option Encode Helpers

Status: implemented

This record preserves the completed `Dict<String, Vec<Option<scalar>>>`
format-neutral encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields include
`Dict<String, Vec<Option<Int>>>`, `Dict<String, Vec<Option<Bool>>>`,
`Dict<String, Vec<Option<Float>>>`, or
`Dict<String, Vec<Option<String>>>`.

The helper accepts the schema-local visible record shape, returns
`Result<T, String>`, and preserves the supplied record on success without
producing binary bytes. Dictionary keys remain restricted to `String`.

This slice does not add arbitrary recursive dictionary or vector encode
eligibility. Shapes such as `Dict<String, Vec<List<Int>>>` and non-string
dictionary keys remain outside the generated encode helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-dict-vec-option-encode/`
  checks successful direct helper calls and explicit schema encode expressions
  over `Dict<String, Vec<Option<Int>>>` fields with non-empty vectors
  containing `Some` and `None`.
- `../../../examples/specification/check/format-neutral-schema-dict-vec-encode-boundary/`
  checks that `Dict<String, Vec<List<Int>>>` remains outside the generated
  encode helper boundary and reports `schema.format_neutral_encode_helper`.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  generated helper resolution for all supported scalar option-vector
  dictionary values plus adjacent rejected dictionary boundaries.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
encode shapes, and later schema composition surfaces.
