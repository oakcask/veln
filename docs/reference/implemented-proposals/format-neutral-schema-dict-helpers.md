# Format-Neutral Schema Dict Helpers

Status: implemented

This record preserves the completed format-neutral top-level string-keyed
dictionary generated helper slices from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`, `../../specification/execution.md`,
and `../../specification/names-effects.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is `Dict<String, Int>` or
`Dict<String, String>`. The helper remains a validation/pass-through boundary
over the schema-local visible record shape and returns
`Result<TRecord, String>`.

The slice does not add general dictionary eligibility. Non-`String` keys,
values outside `Int` or `String`, nested dictionaries, `Option<Dict<...>>`,
and dictionary fields inside nested record-shaped fields remain unsupported
helper fields and keep the `schema.format_neutral_decode_helper` diagnostic
family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful top-level `Dict<String, Int>` and `Dict<String, String>` fields
  beside the existing scalar, list, nested record-shaped, and supported
  `Option` fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported dictionary key, value, nested,
  option-contained, and record-contained shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary format-neutral containers, and
later schema composition surfaces.
