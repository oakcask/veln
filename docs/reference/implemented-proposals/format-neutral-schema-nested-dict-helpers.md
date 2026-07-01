# Format-Neutral Schema Nested Dict Helpers

Status: implemented

This record preserves the completed format-neutral nested record dictionary
generated helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when nested record-shaped fields contain
`Dict<String, Int>`, `Dict<String, Bool>`, `Dict<String, Float>`, or
`Dict<String, String>` fields. The helper remains a validation/pass-through
boundary over the schema-local visible record shape and returns
`Result<TRecord, String>`.

The slice does not add arbitrary nested dictionary eligibility. Nested
dictionaries, `Option<Dict<...>>`, dictionaries inside lists, lists of
dictionaries, and non-string dictionary keys remain unsupported helper fields
and keep the `schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful nested record-shaped fields containing `Dict<String, Int>`,
  `Dict<String, Bool>`, `Dict<String, Float>`, and `Dict<String, String>`
  fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported adjacent dictionary shapes, including
  `Option<Dict<String, Int>>`, `Dict<String, Dict<String, Int>>`, and nested
  record fields containing `Dict<String, Dict<String, Int>>`.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary format-neutral containers,
and later schema composition surfaces.
