# Format-Neutral Schema Option Helpers

Status: implemented

This record preserves the completed format-neutral `Option<T>` generated
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when fields are scalar values, nested
record-shaped values made from scalar or `Option<scalar>` fields, or
`Option<T>` where `T` is one of those scalar or nested record shapes.

The helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns `Result<TRecord, String>`. Unsupported
payloads, including collection payloads such as `Option<List<Int>>` and
nested record fields such as `Option<Dict<String, Int>>`, keep the
`schema.format_neutral_decode_helper` diagnostic family at the field
declaration with a related generated-helper boundary note.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful scalar, nested record-shaped, `Option` scalar, and `Option`
  nested record-shaped fields, plus `Option<scalar>` fields inside a nested
  record-shaped field.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  checks unsupported top-level and nested-record `Option<List<Int>>` payload
  diagnostics.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, format-neutral fields outside scalar,
top-level scalar list, top-level `Dict<String, Int>`, `Dict<String, Bool>`,
`Dict<String, Float>`, or `Dict<String, String>`, supported `Option`, and
nested record-shaped payloads with scalar or `Option<scalar>` fields, and
later schema composition surfaces.
