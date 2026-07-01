# Format-Neutral Schema List Helpers

Status: implemented

This record preserves the completed format-neutral top-level list generated
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is `List<Int>`,
`List<Bool>`, `List<Float>`, or `List<String>`. The helper remains a
validation/pass-through boundary over the schema-local visible record shape and
returns `Result<TRecord, String>`.

The slice does not add general container eligibility. Later work added
top-level `Option<List<scalar>>` fields. Nested lists, nested record fields
that contain nested lists or option-wrapped lists, and `Vec` remain
unsupported helper fields and keep the `schema.format_neutral_decode_helper`
diagnostic family. Later completed records describe additional implemented
format-neutral helper slices.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful top-level `List<Int>`, `List<Bool>`, `List<Float>`, and
  `List<String>` fields beside the existing scalar, nested record-shaped, and
  supported `Option` fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported nested lists, nested record-contained
  nested lists, nested record-contained option-wrapped lists, and unrelated
  container shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary format-neutral containers, and
later schema composition surfaces.
