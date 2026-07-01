# Format-Neutral Schema Option Helpers

Status: implemented

This record preserves the completed format-neutral `Option<T>` generated
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when fields are scalar values, nested
record-shaped values made from those scalar fields, or `Option<T>` where `T`
is one of those scalar or nested record shapes.

The helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns `Result<TRecord, String>`. Unsupported
payloads, including collection payloads such as `Option<List<Int>>`, keep the
`schema.format_neutral_decode_helper` diagnostic family at the field
declaration with a related generated-helper boundary note.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful scalar, nested record-shaped, `Option` scalar, and `Option`
  nested record-shaped fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  checks the unsupported `Option<List<Int>>` payload diagnostic.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, format-neutral fields outside scalar,
supported `Option`, and nested record-shaped payloads, and later schema
composition surfaces.
