# Format-Neutral Schema Encode Helper Diagnostics

Status: implemented

This record preserves the completed format-neutral encode helper diagnostic
cleanup slice from `schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Unsupported format-neutral `byte_encode_<schema>` helper fields use
`schema.format_neutral_encode_helper`. The primary message names the failed
schema field and unsupported field type as the failed fact. The related
`schema_helper_boundary` note describes the recursive format-neutral visible
shape boundary over scalar leaves, anonymous records, supported containers,
and eligible same-module or public imported source ADT fields, with every
child and constructor payload subject to the same rule.

This slice did not expand encode helper eligibility. It only aligned the
unsupported helper diagnostic with the already implemented helper boundary.

## Evidence

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

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
