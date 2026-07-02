# Format-Neutral Schema Option List Helpers

Status: implemented

This record preserves the completed format-neutral top-level
`Option<List<scalar>>` generated helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when a top-level field is
`Option<List<Int>>`, `Option<List<Bool>>`, `Option<List<Float>>`, or
`Option<List<String>>`. The helper remains a validation/pass-through boundary
over the schema-local visible record shape and returns
`Result<TRecord, String>`.

The slice did not add recursive container eligibility. Nested record fields
were completed by the later nested option-list slice, and broader container
recursion was completed by the recursive container helper slice. `Vec<T>` and
binary schema helper behavior remain outside this slice.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-option-list-decode/`
  checks successful top-level `Option<List<Int>>`, `Option<List<Bool>>`,
  `Option<List<Float>>`, and `Option<List<String>>` fields, including present
  and absent option payloads.
- `format-neutral-schema-recursive-container-helpers.md` carries the current
  adjacent negative evidence for unsupported format-neutral helper shapes.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for the accepted top-level option-list
  shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
