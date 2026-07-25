# Format-Neutral Schema Nested Dict Helpers

Status: implemented

This record preserves the completed format-neutral nested record dictionary
generated helper slice from `schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when nested record-shaped fields contain
`Dict<String, Int>`, `Dict<String, Bool>`, `Dict<String, Float>`, or
`Dict<String, String>` fields. The helper remains a validation/pass-through
boundary over the schema-local visible record shape and returns
`Result<TRecord, String>`.

The slice did not add arbitrary nested dictionary eligibility. Later work
added `Option<Dict<String, scalar>>` fields and then generalized
string-keyed dictionary values through the recursive container helper slice.
Non-string dictionary keys remain unsupported helper fields and keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful nested record-shaped fields containing `Dict<String, Int>`,
  `Dict<String, Bool>`, `Dict<String, Float>`, and `Dict<String, String>`
  fields.
- `format-neutral-schema-recursive-container-helpers.md` carries the current
  adjacent negative evidence for unsupported format-neutral helper shapes.

## Remaining Work

Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary field
families outside the implemented helper slices are separate proposals or
explicit non-goals.
