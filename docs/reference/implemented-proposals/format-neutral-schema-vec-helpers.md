# Format-Neutral Schema Vec Helpers

Status: implemented

This record preserves the completed format-neutral `Vec<T>` visible-shape
helper slice from `../../proposals/schema-declaration-surface.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and checked examples.

## Outcome

Format-neutral schemas without a `format` clause may expose generated
`byte_decode_<schema>` helpers when `Vec<T>` fields use an element type that is
accepted by the same recursive format-neutral visible-shape rule. `Vec<T>` is
accepted in the same positions as other supported recursive containers:
top-level schema fields, anonymous record fields, `Option<T>`,
`Dict<String, T>`, `Result<Ok, Err>` payloads, and eligible source ADT
constructor payloads.

The helper remains a schema-local visible record pass-through boundary. It
accepts and returns the record shape containing the `Vec<T>` value.
Unsupported element types, such as function values inside a `Vec`, keep the
`schema.format_neutral_decode_helper` diagnostic family.

## Evidence

- `../../../examples/specification/check/format-neutral-schema-vec-fields/`
  checks accepted top-level and nested `Vec<T>` schema fields.
- `../../../examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`
  checks rejected `Vec<T>` fields whose element type is outside the recursive
  visible-shape boundary.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs`
  checks helper signature generation, IR generation, and unsupported element
  diagnostics for the same boundary.

## Remaining Work

The broader schema declaration proposal remains open for arbitrary recursive
format-neutral encode shapes, binary schema fields outside the implemented
helper slices, and later schema composition surfaces.
