# Format-Neutral Schema Source ADT Helpers

Status: implemented

This record preserves the completed format-neutral source ADT visible-shape
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when fields use same-module source ADT types
whose constructor payloads are recursive format-neutral visible shapes. Source
ADT fields are accepted anywhere other recursive visible shapes are accepted:
top-level schema fields, anonymous record fields, `Option<T>`, `List<T>`,
`Dict<String, T>`, and `Result<Ok, Err>` payloads.

The helper remains a schema-local visible record pass-through boundary. It
accepts and returns the record shape containing the source ADT values.
Unsupported ADT payloads, function types, `Vec<T>`, non-string dictionary
keys, unresolved types, and imported source ADTs keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/check/format-neutral-schema-source-adt-fields/`
  checks accepted same-module source ADT fields in every supported position.
- `../../../examples/specification/check/format-neutral-schema-source-adt-helper-diagnostics/`
  checks rejected source ADT fields with unsupported payloads and imported
  source ADT references.
- `../../../examples/specification/run/format-neutral-schema-source-adt-decode/`
  checks that the generated helper exposes source ADT values through the
  source-visible record boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks helper signature generation, IR generation, and unsupported payload
  diagnostics for the same boundary.

## Remaining Work

The broader schema declaration proposal remains open for imported source ADT
support, format-neutral encode helpers, binary schema fields outside the
implemented helper slices, and later schema composition surfaces.
