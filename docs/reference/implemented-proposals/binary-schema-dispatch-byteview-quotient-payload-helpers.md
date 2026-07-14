# Binary Schema Dispatch ByteView Quotient Payload Helpers

Status: implemented

This record preserves the completed nested dispatch payload helper slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept same-module nested
payload schemas in `Dispatch(...)` and `ExtensionDispatch(...)` when the
nested schema exposes the generated helper path and contains a quotient-sized
`ByteView(left_length / right_length)` field. Both operands must be earlier
visible `Int` fields in that nested schema.

Known dispatch cases decode by running the nested generated helper over the
payload bytes selected by the enclosing dispatch field. Generated decode-step
helpers and `derive decode` use the same nested helper path. Known dispatch
cases encode by running the nested generated encode helper and then applying
the existing dispatch payload length checks; `derive encode` uses that same
generated encode helper boundary. Unknown `ExtensionDispatch` tags preserve
bounded raw payload bytes and do not attempt nested decoding.

Division by zero in the nested `ByteView` length reports
`schema.length_division_by_zero` at the nested field path under the enclosing
dispatch payload field. Encode rejects byte views whose count does not match
the quotient expression with the existing `codec.encode_value_unrepresentable`
shape.

Missing, forward, or non-`Int` operands remain outside the nested helper slice
and keep the focused `schema.dispatch_payload` helper-boundary diagnostic with
the nested schema field path and unsupported `ByteView` layout fact.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-decode/`
  checks same-module closed and extension dispatch decode through a nested
  payload schema with `ByteView(left_length / right_length)`, including the
  generated decode-step helper and derived decode codec boundary.
- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-encode/`
  checks same-module closed and extension dispatch encode through that nested
  helper, including the derived encode codec boundary.
- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-division-by-zero-json/`
  checks that nested quotient division by zero reports
  `schema.length_division_by_zero` with the nested schema field path.
- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-encode-length-mismatch/`
  checks that nested quotient encode rejects a byte-view count mismatch.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
  checks that missing, forward, and non-`Int` nested `ByteView` length
  references still report focused `schema.dispatch_payload` diagnostics.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`
  checks the human diagnostic route for the same helper eligibility boundary.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
dispatch payload schemas outside the generated helper vocabulary, recursive
payload forms outside the selected length-bounded slices, and mapping behavior
outside the implemented structural slices.
