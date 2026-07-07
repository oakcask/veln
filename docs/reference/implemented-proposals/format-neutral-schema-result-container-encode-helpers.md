# Format-Neutral Schema Result Container Encode Helpers

Status: implemented

This record preserves the completed format-neutral result-container encode
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when fields are `List<Result<Ok, Err>>`,
`Vec<Result<Ok, Err>>`, or `Dict<String, Result<Ok, Err>>`, and when the same
shapes appear inside anonymous record fields. Each `Ok` and `Err` payload must
be an existing supported format-neutral encode shape.

The helper remains a validation/pass-through boundary over the supplied
schema-local visible record shape. It returns `Result<TRecord, String>` and
does not produce binary bytes.

This slice does not add arbitrary recursive container encode eligibility.
Function payloads, non-string dictionary keys, and result payloads that depend
on the newly added result-container shape remain outside the generated encode
helper boundary.

## Evidence

- `../../../examples/specification/check/format-neutral-schema-result-container-encode-fields/case.toml`
  checks direct helper and explicit encode expression resolution for accepted
  `List<Result<Ok, Err>>`, `Vec<Result<Ok, Err>>`,
  `Dict<String, Result<Ok, Err>>`, and anonymous record fields.
- `../../../examples/specification/run/format-neutral-schema-result-container-encode/case.toml`
  checks that direct and explicit encode paths preserve the schema-local
  visible record through the generated helper boundary.
- `../../../examples/specification/check/format-neutral-schema-result-container-encode-boundary/case.toml`
  checks that unsupported function payloads, non-string dictionary keys, and
  recursive result-container payloads remain outside the helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper IR lowering for the accepted result-container
  encode shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, format-neutral encode shapes beyond the
implemented supported-shape boundary, and later schema composition surfaces.
