# Binary Schema Primitives And Dispatch

Status: implemented

## Outcome

Binary schemas support the selected exact-width unsigned, reserved-bit,
bounded `ByteView`, bounded `Repeat`, nested-schema, closed-dispatch, and
extension-dispatch layouts required by the checked protocol examples.
Generated helpers operate on schema-local visible records and preserve
representation-only facts for validation and diagnostics.

Current syntax and behavior are specified under
`../../specification/source-surface.md` and
`../../specification/execution.md`. Schema-level `map to` projection is not
part of the implemented surface.

## Evidence

Executable coverage lives in the binary-schema cases under
`../../../examples/specification/check/` and
`../../../examples/specification/run/`. Focused records in this directory
retain the completion evidence for each primitive, reserved layout, repeat,
byte-view, nested payload, dispatch, and diagnostic slice.

## Boundary

This proposal does not imply arbitrary bitstreams or an open-ended primitive
sequence. Signed integers, floating-point encodings, variable-length integers,
text encodings, or a new layout family require a concrete protocol need and a
separate bounded proposal.
