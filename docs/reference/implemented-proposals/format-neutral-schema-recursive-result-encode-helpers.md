# Format-Neutral Schema Recursive Result Encode Helpers

Status: implemented

This record preserves the completed format-neutral recursive `Result` encode
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value`
expressions when `Result<Ok, Err>` fields have payloads that are supported
format-neutral encode shapes. The supported payloads include the existing
scalar, option, container, string-keyed dictionary, anonymous record, and
same-module or public imported source ADT encode shapes.

The helper remains a validation/pass-through boundary over the supplied
schema-local visible record shape. It returns `Result<TRecord, String>` and
does not produce binary bytes.

Unsupported result payloads, including function types, non-string dictionary
keys, unresolved or private imported source ADTs, and source ADTs with
unsupported constructor payloads, remain outside the generated encode helper
boundary.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-recursive-result-encode/`
  checks successful generated helper and explicit encode expression resolution
  for `Result<Option<Int>, String>`,
  `Result<Vec<Int>, Dict<String, String>>`, and nested recursive `Result`
  fields.
- `../../../examples/specification/check/format-neutral-schema-recursive-result-encode-boundary/`
  checks that unsupported result payloads still stay outside the generated
  helper boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper IR lowering for direct and explicit encode paths,
  source ADT result payloads, and unsupported result payload rejection.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices, format-neutral encode shapes beyond the
implemented supported-shape boundary, and later schema composition surfaces.
