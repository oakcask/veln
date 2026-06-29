# Binary Schema Dispatch ByteView Add Payload Helpers

Status: implemented

This record preserves the completed nested dispatch payload helper slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept same-module nested
payload schemas in `Dispatch(...)` and `ExtensionDispatch(...)` when the
nested schema exposes the generated helper path and contains a length-bounded
`ByteView(left_length + right_length)` field. Both operands must be earlier
visible `Int` fields in that nested schema.

Known dispatch cases decode by running the nested generated helper over the
payload bytes selected by the enclosing dispatch field. Known dispatch cases
encode by running the nested generated encode helper and then applying the
existing dispatch payload length checks. Unknown `ExtensionDispatch` tags
preserve bounded raw payload bytes and do not attempt nested decoding.

Missing, forward, or non-`Int` operands remain outside the nested helper slice
and keep the focused `schema.dispatch_payload` helper-boundary diagnostic with
the nested schema field path and unsupported `ByteView` layout fact.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-add-decode/`
  checks same-module closed and extension dispatch decode through a nested
  payload schema with `ByteView(left_length + right_length)`.
- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-add-encode/`
  checks same-module closed and extension dispatch encode through that nested
  helper.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
  checks that missing, forward, and non-`Int` nested `ByteView` length
  references still report focused `schema.dispatch_payload` diagnostics.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`
  checks the human diagnostic route for the same helper eligibility boundary.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
quotient nested dispatch `ByteView` payload helper slices, dispatch payload
schemas outside the generated helper vocabulary, recursive payload forms
outside the selected length-bounded slices, and mapping behavior outside the
implemented structural slices.
