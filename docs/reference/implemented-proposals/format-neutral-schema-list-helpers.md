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

The slice did not add general container eligibility. Later completed records
describe additional implemented format-neutral helper slices, including the
recursive container helper boundary. `Vec` remains outside the format-neutral
schema helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful top-level `List<Int>`, `List<Bool>`, `List<Float>`, and
  `List<String>` fields beside the existing scalar, nested record-shaped, and
  supported `Option` fields.
- `format-neutral-schema-recursive-container-helpers.md` carries the current
  adjacent negative evidence for unsupported format-neutral helper shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
