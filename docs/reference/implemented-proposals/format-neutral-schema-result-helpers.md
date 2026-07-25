# Format-Neutral Schema Result Helpers

Status: implemented

This record preserves the completed format-neutral
`Result<scalar, scalar>` generated helper slice from
`schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is
`Result<Ok, Err>` and both payloads are `Int`, `Bool`, `Float`, or `String`.
Record-shaped fields may contain the same result shapes. The helper remains a
validation/pass-through boundary over the schema-local visible record shape
and returns `Result<TRecord, String>`.

This slice did not add arbitrary result eligibility beyond scalar payloads.
Later work extended `Result` payload eligibility to recursive visible shapes.
Functions, unsupported named types, and shapes such as non-string dictionary
keys remain unsupported helper fields and keep the
`schema.format_neutral_decode_helper` diagnostic family. Later work accepts
same-module and public imported source ADTs when their constructor payloads are
recursive visible shapes, as recorded in
[Format-Neutral Schema Source ADT Helpers](format-neutral-schema-source-adt-helpers.md),
and accepts `Vec<T>` when its element is a recursive visible shape, as recorded
in [Format-Neutral Schema Vec Helpers](format-neutral-schema-vec-helpers.md).

## Evidence

- `../../../examples/specification/run/format-neutral-schema-result-decode/`
  checks successful top-level and record-shaped `Result<scalar, scalar>`
  fields, including source-visible `Ok` and `Err` payloads.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported adjacent result payload shapes, including
  non-string dictionary keys, unsupported source ADT payloads, callbacks, and
  unsupported `Vec<T>` element shapes.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted top-level and record-shaped
  result fields, plus rejection of unsupported result payloads.

## Remaining Work

Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary field
families outside the implemented helper slices are separate proposals or
explicit non-goals.
