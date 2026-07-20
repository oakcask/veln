# Format-Neutral Schema Option Dict Encode Helpers

Status: implemented

This record preserves the completed `Option<Dict<String, scalar>>`
format-neutral encode helper slice from
`schema-declaration-surface.md`. Current behavior is specified
by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when their schema-local visible record fields include
`Option<Dict<String, Int>>`, `Option<Dict<String, Bool>>`,
`Option<Dict<String, Float>>`, or `Option<Dict<String, String>>`. Anonymous
record fields may contain the same option-dictionary scalar shapes.

The helper remains a validation/pass-through boundary over the supplied
schema-local visible record shape. It returns `Result<TRecord, String>` and
does not produce binary bytes.

At this historical slice, arbitrary recursive format-neutral encode eligibility
was not yet available.
Shapes such as `Dict<String, Option<Int>>`,
`Option<Dict<String, Option<Int>>>`, `Option<Dict<String, List<Int>>>`, and
non-string dictionary keys were outside the generated encode helper surface at
this slice. Later dictionary-option encode helper support is tracked in a
sibling implemented proposal record.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-option-dict-encode/`
  checks successful direct helper calls and explicit schema encode expressions
  over top-level and anonymous-record `Option<Dict<String, scalar>>` fields,
  including present and absent option payloads.
- `../../../examples/specification/check/format-neutral-schema-option-dict-encode-boundary/`
  checks that non-string dictionary keys remain outside the generated encode
  helper boundary while nested eligible dictionary values are accepted.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  generated helper resolution for accepted top-level and record-shaped
  option-dictionary encode fields plus the current recursive dictionary
  boundary.

## Superseding Work

The completed recursive eligibility rule is recorded in
[Recursive Format-Neutral Schema Encode Shapes](recursive-format-neutral-schema-encode-shapes.md).
The broader schema declaration proposal remains open only for its binary
helper and later schema-composition work.
