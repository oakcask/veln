# Binary Schema Directional Dispatch Payload Helpers

Status: implemented

This record preserves the completed direction-specific nested dispatch payload
helper eligibility slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Parent binary schema decode helpers, generated decode-step helpers, and
`derive decode` accept a nested dispatch payload schema when the nested schema
exposes the generated decode helper. Decode eligibility no longer depends on
whether the nested payload can also expose the generated encode helper.

Generated encode helpers and `derive encode` still require the nested payload
schema to expose the generated encode helper. A mapped payload schema whose
decoded mapping shape is available but whose mapping assignment cannot project
back to schema-local encode fields remains rejected with the focused
`schema.dispatch_payload` helper-boundary diagnostic on encode paths.

Payload schemas that cannot expose the generated decode helper remain
decode-ineligible and keep the existing `schema.dispatch_payload` diagnostics,
including missing, private, wrong-kind, forward, non-binary, unsupported
`ReservedBits`, unsupported `ByteView` length-reference, and unbounded
recursive payload cases.

## Evidence

- `../../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`
  keeps decode-ineligible payload schemas on the existing dispatch payload
  diagnostic route.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
unsupported field layouts, recursive forms outside the selected length-bounded
slices, and schema value mapping outside the implemented structural slices.
