# Format-Neutral Schema Result Helpers

Status: implemented

This record preserves the completed format-neutral
`Result<scalar, scalar>` generated helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is
`Result<Ok, Err>` and both payloads are `Int`, `Bool`, `Float`, or `String`.
Record-shaped fields may contain the same result shapes. The helper remains a
validation/pass-through boundary over the schema-local visible record shape
and returns `Result<TRecord, String>`.

The slice does not add arbitrary result eligibility. Result payloads that are
lists, dictionaries, records, options, ADTs, functions, or other non-scalar
shapes remain unsupported helper fields and keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-result-decode/`
  checks successful top-level and record-shaped `Result<scalar, scalar>`
  fields, including source-visible `Ok` and `Err` payloads.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported adjacent result payload shapes, including
  `Result<List<Int>, String>` and `Result<Int, Dict<String, String>>`.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted top-level and record-shaped
  result fields, plus rejection of unsupported result payloads.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
