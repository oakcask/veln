# Format-Neutral Schema Nested List Helpers

Status: implemented

This record preserves the completed format-neutral nested record list
generated helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when nested record-shaped fields contain
`List<Int>`, `List<Bool>`, `List<Float>`, or `List<String>` fields. The
helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns `Result<TRecord, String>`.

The slice does not add arbitrary nested collection eligibility. Nested
dictionaries, recursive option-list payloads such as
`Option<List<List<T>>>`, nested `List<Option<T>>`, and nested lists of records
remain unsupported helper fields and keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful nested record-shaped fields containing `List<Int>`, `List<Bool>`,
  `List<Float>`, and `List<String>` fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported nested collection shapes, including nested
  record fields containing nested lists and recursive option-list payloads.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary format-neutral containers,
and later schema composition surfaces.
