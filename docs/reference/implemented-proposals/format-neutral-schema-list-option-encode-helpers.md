# Format-Neutral Schema List Option Encode Helpers

Status: implemented

This record preserves the completed `List<Option<scalar>>` format-neutral
encode helper slice from `../../proposals/schema-declaration-surface.md`.
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

This slice does not add arbitrary recursive `List<Option<T>>` encode
eligibility. Shapes such as `List<Option<List<Int>>>`,
`List<Option<Dict<String, Int>>>`, and
`List<Option<Result<Int, String>>>` remain outside the generated encode helper
boundary. Non-string dictionary keys remain outside the format-neutral visible
shape boundary and are covered by existing dictionary boundary examples.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-list-option-encode/`
  checks direct helper calls and explicit schema encode expressions over
  top-level and anonymous record `List<Option<scalar>>` fields.
- `../../../examples/specification/check/format-neutral-schema-list-option-encode-boundary/`
  checks adjacent non-scalar option payloads inside `List<Option<T>>` stay
  outside the generated encode helper boundary.
- `../../../examples/specification/check/format-neutral-schema-dict-scalar-encode-boundary/`
  keeps non-string dictionary keys outside the format-neutral helper surface.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  helper resolution, Core lowering, and IR lowering for all supported scalar
  option list fields.

## Remaining Work

The broader schema declaration proposal remains open for format-neutral encode
helpers beyond the implemented scalar, supported container, dictionary,
result, anonymous record, and source ADT shapes, binary schema fields outside
the implemented helper slices, and later schema composition surfaces.
