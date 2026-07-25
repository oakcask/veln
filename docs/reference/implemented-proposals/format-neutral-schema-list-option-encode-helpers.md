# Format-Neutral Schema List Option Encode Helpers

Status: implemented

This record preserves the completed `List<Option<scalar>>` format-neutral
encode helper slice from `schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields include
`List<Option<Int>>`, `List<Option<Bool>>`, `List<Option<Float>>`, or
`List<Option<String>>`.

The same shape is supported inside anonymous record fields when every
enclosing field remains a supported format-neutral encode shape. The helper
accepts the schema-local visible record shape, returns `Result<TRecord,
String>`, and preserves the supplied record on success without producing
binary bytes.

At this historical slice, arbitrary recursive `List<Option<T>>` encode
eligibility was not yet available. A later completed slice added the bounded
`List<Option<List<scalar>>>` shape and is archived under
[Format-Neutral Schema List Option List Encode Helpers](format-neutral-schema-list-option-list-encode-helpers.md).
Non-string dictionary keys remain outside the format-neutral visible shape
boundary and are covered by existing dictionary boundary examples.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-list-option-encode/`
  checks direct helper calls and explicit schema encode expressions over
  top-level and anonymous record `List<Option<scalar>>` fields, and later also
  checks the bounded `List<Option<List<scalar>>>` slice.
- `../../../examples/specification/check/format-neutral-schema-dict-scalar-encode-boundary/`
  keeps non-string dictionary keys outside the format-neutral helper surface.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution, Core lowering, and IR lowering for all supported scalar
  option list fields.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
Schema composition is complete under
[Schema Declaration Surface](schema-declaration-surface.md). Binary helper
families outside the implemented slices are separate proposals or explicit
non-goals.
