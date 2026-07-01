# Format-Neutral Schema List Helpers

Status: implemented

This record preserves the completed format-neutral `List<Int>` generated
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is `List<Int>`. The
helper remains a validation/pass-through boundary over the schema-local visible
record shape and returns `Result<TRecord, String>`.

The slice does not add general container eligibility. Payloads such as
`Option<List<Int>>`, other `List<T>` element types, nested record fields that
contain lists, `Vec`, and `Dict` remain unsupported helper fields and keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks a
  successful top-level `List<Int>` field beside the existing scalar, nested
  record-shaped, and supported `Option` fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported `Option<List<Int>>`, other list payloads,
  nested record-contained lists, and unrelated container shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary format-neutral containers, and
later schema composition surfaces.
