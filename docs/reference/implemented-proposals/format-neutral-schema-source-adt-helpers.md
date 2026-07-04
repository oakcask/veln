# Format-Neutral Schema Source ADT Helpers

Status: implemented

This record preserves the completed format-neutral source ADT helper slice
from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when fields use same-module source ADT types or
public imported source ADT types referenced through written `use` paths whose
constructor payloads are recursive format-neutral visible shapes. Source ADT
fields are accepted anywhere other recursive visible shapes are accepted:
top-level schema fields, anonymous record fields, `Option<T>`, `List<T>`,
`Vec<T>`, `Dict<String, T>`, and `Result<Ok, Err>` payloads.

The helper remains a schema-local visible record pass-through boundary. It
accepts and returns the record shape containing the source ADT values.
Unsupported ADT payloads, function types, non-string dictionary keys,
unresolved types, private imported source ADTs, missing paths, and non-ADT
targets keep the `schema.format_neutral_decode_helper` diagnostic family.

Format-neutral schemas without a `format` clause may also expose generated
`byte_encode_<schema>` helpers and explicit `encode Schema from value` support
for same-module source ADT fields and public imported source ADT fields
referenced through written `use` paths. Encode support is narrower than decode:
the source ADT constructor payloads must be supported format-neutral encode
shapes. Unsupported constructor payloads, private imported source ADTs, missing
paths, and non-ADT targets use the `schema.format_neutral_encode_helper`
diagnostic family when the matching decode boundary is also unsupported.

## Evidence

- `../../../examples/specification/check/format-neutral-schema-source-adt-fields/`
  checks accepted same-module source ADT fields in every supported position
  and an accepted public imported source ADT field.
- `../../../examples/specification/check/format-neutral-schema-source-adt-helper-diagnostics/`
  checks rejected source ADT fields with unsupported payloads, private imported
  source ADT references, and imported source ADTs with unsupported payloads.
- `../../../examples/specification/run/format-neutral-schema-source-adt-decode/`
  checks that the generated helper exposes same-module and imported public
  source ADT values through the source-visible record boundary.
- `../../../examples/specification/run/format-neutral-schema-source-adt-encode/`
  checks that generated direct and explicit encode helpers preserve
  same-module and imported public source ADT values through the
  source-visible record boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks helper signature generation, IR generation, and unsupported payload
  diagnostics for the decode and encode boundaries.

## Remaining Work

The broader schema declaration proposal remains open for arbitrary recursive
format-neutral encode shapes, binary schema fields outside the implemented
helper slices, and later schema composition surfaces.
