# Format-Neutral Schema Recursive Container Helpers

Status: implemented

This record preserves the completed recursive format-neutral container
generated helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md` and
`../../specification/execution.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when fields use recursive visible shapes made
from scalar leaves, anonymous record fields, `Option<T>`, `List<T>`, and
`Dict<String, T>`. The same rule applies to top-level schema fields and fields
inside anonymous record shapes. The helper remains a validation/pass-through
boundary over the schema-local visible record shape and returns
`Result<TRecord, String>`.

This slice did not extend `Result<Ok, Err>` beyond scalar payloads. Later
work extended `Result` payload eligibility to recursive visible shapes.
Non-string dictionary keys, function types, and unsupported named types remain
unsupported helper fields and keep the `schema.format_neutral_decode_helper`
diagnostic family. Later work accepts same-module source ADTs when their
constructor payloads are recursive visible shapes, as recorded in
[Format-Neutral Schema Source ADT Helpers](format-neutral-schema-source-adt-helpers.md).

## Evidence

- `../../../examples/specification/run/format-neutral-schema-recursive-containers-decode/`
  checks successful `List<Option<Int>>`,
  `Option<List<Option<String>>>`, and `Dict<String, Option<Int>>` fields at
  the top level and inside an anonymous record shape.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported adjacent shapes, including non-string
  dictionary keys, function types, unsupported named types, and unsupported
  recursive result payloads.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for accepted recursive containers and
  rejection of unsupported adjacent shapes.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
