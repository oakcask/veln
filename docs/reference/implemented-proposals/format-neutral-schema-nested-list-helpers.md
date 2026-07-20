# Format-Neutral Schema Nested List Helpers

Status: implemented

This record preserves the completed format-neutral nested record list
generated helper slice from `schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when nested record-shaped fields contain
`List<Int>`, `List<Bool>`, `List<Float>`, or `List<String>` fields. The
helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns `Result<TRecord, String>`.

The slice did not add arbitrary nested collection eligibility. Broader
container recursion was completed by the recursive container helper slice.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful nested record-shaped fields containing `List<Int>`, `List<Bool>`,
  `List<Float>`, and `List<String>` fields.
- `format-neutral-schema-recursive-container-helpers.md` carries the current
  adjacent negative evidence for unsupported format-neutral helper shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
