# Format-Neutral Schema Dict Scalar Encode Helpers

Status: implemented

This record preserves the completed `Dict<String, scalar>` format-neutral
encode helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is a scalar leaf,
`Option<scalar>` field, `List<scalar>` field, or `Dict<String, scalar>` field.
The scalar leaves are `Int`, `Bool`, `Float`, and `String`.

The helper accepts the same schema-local visible record shape as the generated
format-neutral decode helper and returns `Result<T, String>`, preserving the
supplied record on success without producing binary bytes. Explicit
`encode Schema from value` expressions use the same helper boundary.

At this slice, recursive format-neutral encode helper shapes beyond the
supported dictionary slice, such as `Dict<String, Option<Int>>`, were still
unsupported and did not expose a generated helper. Dictionaries with
non-`String` keys remain unsupported. Later container and dictionary-option
encode helper slices are tracked in sibling implemented proposal records.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-dict-scalar-encode/`
  checks direct helper calls and explicit schema encode expressions over
  `Dict<String, scalar>` fields.
- `../../../examples/specification/check/format-neutral-schema-dict-scalar-encode-boundary/`
  checks that non-string dictionary keys and unsupported dictionary value
  shapes remain outside the generated encode helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation plus Core and IR lowering.

## Remaining Work

The broader schema declaration proposal remains open for format-neutral encode
helpers beyond the implemented scalar, supported container, scalar-result,
and anonymous record shapes, binary schema fields outside the implemented
helper slices, and later schema composition surfaces.
