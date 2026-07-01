# Format-Neutral Schema Nested Option List Helpers

Status: implemented

This record preserves the completed format-neutral nested record
`Option<List<scalar>>` generated helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when nested record-shaped fields contain
`Option<List<Int>>`, `Option<List<Bool>>`, `Option<List<Float>>`, or
`Option<List<String>>` fields. The helper remains a validation/pass-through
boundary over the schema-local visible record shape and returns
`Result<TRecord, String>`.

The slice does not add arbitrary recursive container eligibility. Deeper
payloads such as `Option<List<List<T>>>`, nested `List<Option<T>>`, nested
lists of records, `Vec<T>`, non-string-keyed dictionaries, and binary schema
helper behavior remain outside this slice.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-nested-option-list-decode/`
  checks successful nested record-shaped fields containing
  `Option<List<Int>>`, `Option<List<Bool>>`, `Option<List<Float>>`, and
  `Option<List<String>>` fields, including present and absent option payloads.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for deeper recursive option-list payloads.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted nested option-list shapes
  and rejection of deeper option-list nesting.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
containers, and later schema composition surfaces.
