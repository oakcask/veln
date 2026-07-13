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

At this historical slice, arbitrary recursive encode eligibility was not yet
available. Shapes such
as `Option<List<List<Int>>>`, `List<Option<Int>>`,
`Dict<String, Option<Int>>`, and `Result<List<Int>, String>` were outside
the generated format-neutral encode helper surface at this slice. Later
dictionary-option and list-option encode helper support is tracked in sibling
implemented proposal records.

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
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  retains helper resolution coverage for these container shapes; deeper mixed
  shapes are covered by the superseding recursive boundary record.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
