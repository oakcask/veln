# Format-Neutral Schema Container Encode Helpers

Status: implemented

This record preserves the completed first format-neutral container encode
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields are built from the
existing scalar encode shapes plus these additional shapes:
`Option<List<scalar>>`, anonymous record fields whose fields are supported
format-neutral encode shapes, and `Result<scalar, scalar>`.

The helper remains a validation/pass-through boundary over the supplied
schema-local visible record shape. It returns `Result<TRecord, String>` and
does not produce binary bytes.

This slice does not add arbitrary recursive encode eligibility. Shapes such
as `Option<List<List<Int>>>`, `List<Option<Int>>`,
`Dict<String, Option<Int>>`, and `Result<List<Int>, String>` remain outside
the generated format-neutral encode helper surface.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-option-list-encode/`
  checks successful `Option<List<scalar>>` fields through the generated helper
  and explicit encode expression.
- `../../../examples/specification/run/format-neutral-schema-nested-container-encode/`
  checks nested anonymous record fields containing `List<scalar>` and
  `Option<List<scalar>>` fields.
- `../../../examples/specification/run/format-neutral-schema-result-scalar-encode/`
  checks successful `Result<scalar, scalar>` fields, including `Ok` and `Err`
  payloads.
- `../../../examples/specification/check/format-neutral-schema-container-encode-boundary/`
  keeps adjacent unsupported recursive container and non-scalar result
  payload shapes outside the encode helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted container encode shapes and
  rejection of a deeper recursive container shape.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
encode shapes, source ADT encode fields, and later schema composition
surfaces.
