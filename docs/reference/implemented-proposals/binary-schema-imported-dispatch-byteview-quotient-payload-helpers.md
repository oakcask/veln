# Binary Schema Imported Dispatch ByteView Quotient Payload Helpers

Status: implemented

This record preserves the completed public imported nested dispatch payload
helper slice from `../../proposals/binary-schema-primitives-and-dispatch.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/names-effects.md`,
and the checked executable examples under
`../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept public imported
nested payload schemas in `Dispatch(...)` and `ExtensionDispatch(...)` when the
nested schema exposes the generated helper path and contains a quotient-sized
`ByteView(left_length / right_length)` field. Both operands must be earlier
visible `Int` fields in the imported nested schema's schema-local visible
record shape.

Known dispatch cases decode by running the imported nested generated helper
over the payload bytes selected by the enclosing dispatch field. Generated
decode-step helpers and explicit schema decode expressions use that same
imported helper path.
Known dispatch cases encode by running the imported nested generated encode
helper and then applying the existing dispatch payload length checks;
explicit schema encode expressions use that same generated encode helper
boundary. Unknown `ExtensionDispatch` tags preserve bounded raw payload bytes
and do not attempt nested decoding.

Division by zero in the imported nested `ByteView` length reports
`schema.length_division_by_zero` at the imported nested field path under the
enclosing dispatch payload field. Encode rejects byte views whose count does
not match the quotient expression with the existing
`schema.encode_value_unrepresentable` shape.

Missing, forward, or non-`Int` operands remain outside the nested helper slice
and keep the focused `schema.dispatch_payload` helper-boundary diagnostic with
the imported nested schema field path and unsupported `ByteView` layout fact.

## Evidence

- `../../../examples/specification/run/binary-schema-imported-dispatch-nested-byteview-quotient-decode/`
  checks closed and extension dispatch decode through a public imported nested
  payload schema with `ByteView(total_length / chunk_count)`, including the
  generated decode-step helper and explicit schema decode expression boundary.
- `../../../examples/specification/run/binary-schema-imported-dispatch-nested-byteview-quotient-encode/`
  checks closed and extension dispatch encode through that imported nested
  helper, including the explicit schema encode expression boundary.
- `../../../examples/specification/run/binary-schema-imported-dispatch-nested-byteview-quotient-division-by-zero-json/`
  checks that imported nested quotient division by zero reports
  `schema.length_division_by_zero` with the imported nested schema field path.
- `../../../examples/specification/run/binary-schema-imported-dispatch-nested-byteview-quotient-encode-length-mismatch/`
  checks that imported nested quotient encode rejects a byte-view count
  mismatch with the imported nested schema field path.
- Same-module quotient decode, encode, division-by-zero, and encode length
  mismatch behavior remains checked by
  `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-decode/`,
  `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-encode/`,
  `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-division-by-zero-json/`,
  and
  `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-quotient-encode-length-mismatch/`.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
dispatch payload schemas outside the generated helper vocabulary, recursive
payload forms outside the selected length-bounded slices, and mapping behavior
outside the implemented structural slices.
