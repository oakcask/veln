# Format-Neutral Schema Option Vec Encode Helpers

Status: implemented

This record preserves the completed `Vec<Option<scalar>>` format-neutral
encode helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is one of the supported
format-neutral encode shapes, including `Vec<Option<scalar>>` fields. The
supported scalar leaves are `Int`, `Bool`, `Float`, and `String`.

The helper accepts the schema-local visible record shape, returns
`Result<T, String>`, and preserves the supplied record on success without
producing binary bytes. Explicit `encode Schema from value` expressions use
the same helper boundary.

This slice did not add recursive `Vec<T>` encode eligibility. The later
bounded `Vec<Vec<scalar>>` slice is recorded in
[Format-Neutral Schema Recursive Vec Scalar Encode Helpers](format-neutral-schema-recursive-vec-scalar-encode-helpers.md).

## Evidence

- `../../../examples/specification/run/format-neutral-schema-option-vec-encode/`
  checks direct helper calls and explicit schema encode expressions over
  `Vec<Option<scalar>>` fields.
- `../../../examples/specification/check/format-neutral-schema-vec-scalar-encode-boundary/`
  keeps `Vec<Vec<Vec<Int>>>` outside the generated encode helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation, Core and IR lowering, bounded recursive `Vec`
  acceptance, and deeper `Vec` rejection.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, arbitrary recursive format-neutral
encode shapes, and later schema composition surfaces.
