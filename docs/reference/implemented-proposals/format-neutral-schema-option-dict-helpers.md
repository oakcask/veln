# Format-Neutral Schema Option Dict Helpers

Status: implemented

This record preserves the completed format-neutral
`Option<Dict<String, scalar>>` generated helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is
`Option<Dict<String, Int>>`, `Option<Dict<String, Bool>>`,
`Option<Dict<String, Float>>`, or `Option<Dict<String, String>>`.
Record-shaped fields may contain the same option dictionary field shapes. The
helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns `Result<TRecord, String>`.

The slice does not add arbitrary dictionary eligibility. Non-string dictionary
keys, dictionary values that are records, dictionaries, lists, options, ADTs,
functions, or other non-scalar shapes, and recursive container shapes remain
unsupported helper fields and keep the `schema.format_neutral_decode_helper`
diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful top-level and record-shaped `Option<Dict<String, Int>>`,
  `Option<Dict<String, Bool>>`, `Option<Dict<String, Float>>`, and
  `Option<Dict<String, String>>` fields, including present and absent option
  payloads.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported adjacent dictionary shapes, including
  non-string dictionary keys and nested dictionaries such as
  `Dict<String, Dict<String, Int>>`.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted top-level and record-shaped
  option-dictionary fields, plus rejection of unsupported dictionary shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
containers, and later schema composition surfaces.
