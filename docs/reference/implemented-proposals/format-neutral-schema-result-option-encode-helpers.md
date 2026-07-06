# Format-Neutral Schema Result Option Encode Helpers

Status: implemented

This record preserves the completed format-neutral
`Result<scalar, Option<scalar>>` encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when top-level fields are `Result<scalar, Option<scalar>>`.
Anonymous record fields may contain the same shape when their other fields are
supported format-neutral encode shapes. The supported scalar leaves remain
`Int`, `Bool`, `Float`, and `String`.

The helper remains a validation/pass-through boundary over the supplied
schema-local visible record shape. It returns `Result<TRecord, String>` and
does not produce binary bytes.

This slice did not add arbitrary recursive result encode eligibility. Shapes
such as `Result<Option<Int>, String>`, nested `Result` payloads, and
container payloads outside the existing supported encode shapes remain outside
the generated format-neutral encode helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-result-option-encode/`
  checks successful generated helper and explicit encode expression
  resolution, top-level and anonymous record field positions, and pass-through
  runtime behavior for `Ok`, `Err(None)`, and `Err(Some(scalar))` values.
- `../../../examples/specification/check/format-neutral-schema-result-option-encode-boundary/`
  keeps the nearby `Result<Option<scalar>, scalar>` shape outside the encode
  helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted result-option fields and
  rejection of the reversed option-payload shape.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
encode shapes, and later schema composition surfaces.
