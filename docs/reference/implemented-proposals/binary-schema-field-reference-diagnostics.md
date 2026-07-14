# Binary Schema Field Reference Diagnostics

Status: implemented

This record preserves the completed schema-local field reference diagnostics
slice from `binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/source-surface.md` and the
checked executable examples under `../../../examples/specification/check/`.

## Outcome

Binary schema field reference checks reject references that cannot name an
earlier decoded visible `Int` field in the same schema. The checks cover repeat
count fields and count expressions, `ByteView` length expressions,
`ByteView` payload multiple operands, closed-dispatch tag fields, and
extension-dispatch tag and length fields.

Missing references and references to later fields report the same focused
primary message for the failed operand, with structured details distinguishing
`unknown_field_reference` from `forward_field_reference`. References to earlier
fields with the wrong decoded type report `incompatible_field_reference` and
name the actual decoded type. When a compatible earlier field exists, the
diagnostic carries related context naming that field.

The diagnostic ids are `schema.repeat_reference`,
`schema.byte_view_reference`, and `schema.dispatch_reference`.

## Evidence

- `../../../examples/specification/check/binary-schema-field-reference-diagnostics/`
  checks machine-readable diagnostic ids, reasons, messages, and related
  context for missing, forward, and incompatible repeat, `ByteView`, and
  dispatch references.
- `../../../examples/specification/check/binary-schema-field-reference-human/`
  checks focused human output and the compatible earlier-field note.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
new primitive and dispatch helper behavior. New helper slices should add their
own focused declaration or runtime diagnostics when they introduce new
schema-local reference forms.
