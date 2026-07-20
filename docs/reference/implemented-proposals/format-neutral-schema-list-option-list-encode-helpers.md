# Format-Neutral Schema List Option List Encode Helpers

Status: implemented

This record preserves the completed `List<Option<List<scalar>>>`
format-neutral encode helper slice from
`schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`, `../../specification/execution.md`,
and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields include
`List<Option<List<Int>>>`, `List<Option<List<Bool>>>`,
`List<Option<List<Float>>>`, or `List<Option<List<String>>>`.

The same shape is supported inside anonymous record fields when every
enclosing field remains a supported format-neutral encode shape. The helper
accepts the schema-local visible record shape, returns `Result<TRecord,
String>`, and preserves the supplied record on success without producing
binary bytes.

At this historical slice, arbitrary recursive `List<Option<T>>` encode
eligibility was not yet available.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-list-option-encode/`
  checks direct helper calls and explicit schema encode expressions over
  top-level and anonymous record `List<Option<List<scalar>>>` fields across
  the supported scalar leaves.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution, Core lowering, and IR lowering for all supported scalar
  list option list fields.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary helper
families outside the implemented slices are separate proposals or explicit
non-goals.
