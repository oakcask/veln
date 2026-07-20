# Format-Neutral Schema Option Helpers

Status: implemented

This record preserves the completed format-neutral `Option<T>` generated
helper slice from `schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when fields are scalar values, nested
record-shaped values made from scalar or `Option<scalar>` fields, or
`Option<T>` where `T` is one of those scalar or nested record shapes.

The helper remains a validation/pass-through boundary over the schema-local
visible record shape and returns `Result<TRecord, String>`. This slice did not
add collection payloads. Later work added top-level `Option<List<scalar>>`
fields, nested record `Option<List<scalar>>` fields, and
`Option<Dict<String, scalar>>` fields, then generalized recursive container
payloads through the recursive container helper slice.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-decode/` checks
  successful scalar, nested record-shaped, `Option` scalar, and `Option`
  nested record-shaped fields, plus `Option<scalar>` fields inside a nested
  record-shaped field.
- `format-neutral-schema-recursive-container-helpers.md` carries the current
  adjacent negative evidence for unsupported format-neutral helper shapes.

## Remaining Work

Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary field
families outside the implemented helper slices are separate proposals or
explicit non-goals. Later completed records describe additional
format-neutral helper slices.
