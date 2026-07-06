# Format-Neutral Schema Encode Helper Diagnostics

Status: implemented

This record preserves the completed format-neutral encode helper diagnostic
cleanup slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Unsupported format-neutral `byte_encode_<schema>` helper fields use
`schema.format_neutral_encode_helper`. The primary message names the failed
schema field and unsupported field type as the failed fact. The related
`schema_helper_boundary` note describes the full supported format-neutral
encode shape boundary: scalar leaves, supported `Option`, `List`, `Vec`,
`Dict`, recursive `Result`, anonymous record fields, and eligible
same-module or public imported source ADT fields.

This slice did not expand encode helper eligibility. It only aligned the
unsupported helper diagnostic with the already implemented helper boundary.

## Evidence

- `../../../examples/specification/check/format-neutral-schema-container-encode-boundary/`
  checks JSON output for an unsupported container encode field, including
  `details` and the helper-boundary related note.
- `../../../examples/specification/check/format-neutral-schema-container-encode-boundary-human/`
  checks human output for the unsupported container field and boundary note.
- `../../../examples/specification/check/format-neutral-schema-source-adt-helper-diagnostics/`
  checks JSON output for unsupported source ADT encode fields, including
  `details` and the helper-boundary related note.
- `../../../examples/specification/check/format-neutral-schema-source-adt-helper-diagnostics-human/`
  checks human output for an unsupported source ADT encode field and boundary
  note.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks that unsupported source ADT encode diagnostics keep the primary
  message focused on the failed field and include the wider helper boundary
  in related context.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, format-neutral encode shapes beyond the
implemented supported-shape boundary, and later schema composition surfaces.
