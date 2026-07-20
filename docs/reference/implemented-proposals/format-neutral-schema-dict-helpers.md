# Format-Neutral Schema Dict Helpers

Status: implemented

This record preserves the completed format-neutral top-level string-keyed
dictionary generated helper slices from
`schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`, `../../specification/execution.md`,
and `../../specification/names-effects.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is `Dict<String, Int>`,
`Dict<String, Bool>`, `Dict<String, Float>`, or `Dict<String, String>`. The
helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns
`Result<TRecord, String>`.

The slice did not add general dictionary eligibility beyond top-level scalar
dictionary fields. Later work extended the same string-keyed scalar dictionary
shape to fields inside nested record-shaped fields and inside
`Option<Dict<String, scalar>>` fields, then generalized string-keyed
dictionary values through the recursive container helper slice. Non-`String`
keys remain unsupported helper fields and keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful top-level `Dict<String, Int>`, `Dict<String, Bool>`,
  `Dict<String, Float>`, and `Dict<String, String>` fields beside the
  existing scalar, list, nested record-shaped, and supported `Option` fields.
- `format-neutral-schema-recursive-container-helpers.md` carries the current
  adjacent negative evidence for unsupported format-neutral helper shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
