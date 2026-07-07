# Format-Neutral Schema List Option List Encode Helpers

Status: implemented

This record preserves the completed `List<Option<List<scalar>>>`
format-neutral encode helper slice from
`../../proposals/schema-declaration-surface.md`. Current behavior is specified
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

This slice does not add arbitrary recursive `List<Option<T>>` encode
eligibility. Shapes such as `List<Option<Dict<String, Int>>>` and
`List<Option<Result<Int, String>>>` remain outside the generated encode helper
boundary.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-list-option-encode/`
  checks direct helper calls and explicit schema encode expressions over
  top-level and anonymous record `List<Option<List<scalar>>>` fields across
  the supported scalar leaves.
- `../../../examples/specification/check/format-neutral-schema-list-option-encode-boundary/`
  keeps adjacent dictionary and result payloads inside `List<Option<T>>`
  outside the generated encode helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution, Core lowering, and IR lowering for all supported scalar
  list option list fields.

## Remaining Work

The broader schema declaration proposal remains open for format-neutral encode
helpers beyond the implemented scalar, supported container, dictionary,
result, anonymous record, and source ADT shapes, binary schema fields outside
the implemented helper slices, and later schema composition surfaces.
