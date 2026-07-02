# Format-Neutral Schema Result Visible Shapes

Status: implemented

This record preserves the completed format-neutral recursive `Result`
visible-shape helper slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and
`../../specification/names-effects.md`.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when `Result<Ok, Err>` fields have payloads
that are both recursive format-neutral visible shapes. Accepted payloads are
made from scalar leaves, anonymous record fields, `Option<T>`, `List<T>`,
`Dict<String, T>`, and nested `Result<Ok, Err>` values that satisfy the same
rule. The same rule applies to top-level schema fields and fields inside
anonymous record shapes. The helper remains a validation/pass-through boundary
over the schema-local visible record shape and returns `Result<TRecord,
String>`.

The slice did not add general support for source-declared ADTs, function
values, `Vec<T>`, unsupported named types, or dictionaries with non-string
keys. Later work accepts same-module and public imported source ADTs when their
constructor payloads are recursive visible shapes, as recorded in
[Format-Neutral Schema Source ADT Helpers](format-neutral-schema-source-adt-helpers.md),
and accepts `Vec<T>` when its element is a recursive visible shape, as recorded
in [Format-Neutral Schema Vec Helpers](format-neutral-schema-vec-helpers.md).

## Evidence

- `../../../examples/specification/run/format-neutral-schema-result-decode/`
  checks successful top-level `Result<List<Int>, String>`,
  `Result<Int, Dict<String, String>>`, nested record fields containing those
  result shapes, and an `Option<Result<List<Int>, String>>` wrapper.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  keeps diagnostics for unsupported adjacent result payload shapes, including
  non-string dictionary keys, unsupported source ADT payloads, callbacks, and
  unsupported `Vec<T>` element shapes.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks generated helper resolution for recursive result payloads and
  rejection of non-visible result payloads.

## Remaining Work

The broader schema declaration proposal remains open for binary schema fields
outside the implemented helper slices and later schema composition surfaces.
