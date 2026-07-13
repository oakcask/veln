# Format-Neutral Schema Option Scalar Encode Helpers

Status: implemented

This record preserves the completed `Option<scalar>` format-neutral encode
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers when every field is a scalar leaf or an
`Option<scalar>` field. The scalar leaves are `Int`, `Bool`, `Float`, and
`String`.

The helper accepts the same schema-local visible record shape as the generated
format-neutral decode helper and returns `Result<T, String>`, preserving the
supplied record on success without producing binary bytes. Explicit
`encode Schema from value` expressions use the same helper boundary.

At this historical slice, format-neutral encode helper shapes beyond scalar leaves and
`Option<scalar>` fields, such as `Option<List<Int>>`, remained unsupported and
did not expose a generated helper. Later `List<scalar>`,
`Dict<String, scalar>`, and first container encode helper slices are tracked
in sibling implemented proposal records.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-option-scalar-encode/`
  checks direct helper calls and explicit schema encode expressions over
  `Option<scalar>` fields.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper signature generation plus Core and IR lowering.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
