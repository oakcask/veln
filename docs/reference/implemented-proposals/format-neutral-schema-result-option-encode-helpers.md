# Format-Neutral Schema Result Option Encode Helpers

Status: implemented

This record preserves the completed format-neutral
`Result<scalar, Option<scalar>>` encode helper slice from
`schema-declaration-surface.md`. Current behavior is specified
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

The later recursive `Result` encode slice widened result payload eligibility
after this slice. That follow-up is recorded in
`format-neutral-schema-recursive-result-encode-helpers.md`.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-result-option-encode/`
  checks successful generated helper and explicit encode expression
  resolution, top-level and anonymous record field positions, and pass-through
  runtime behavior for `Ok`, `Err(None)`, and `Err(Some(scalar))` values.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted result-option fields.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary helper
families outside the implemented slices are separate proposals or explicit
non-goals.
