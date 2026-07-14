# Binary Schema Dispatch ByteView Subtract Payload Helpers

Status: implemented

This record preserves the completed nested dispatch payload helper slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept same-module and
public imported nested payload schemas in `Dispatch(...)` and
`ExtensionDispatch(...)` when the nested schema exposes the generated helper
path and contains a subtractive `ByteView(left_length - right_length)` field.
Both operands must be earlier visible `Int` fields in that nested schema.

Known dispatch cases decode by running the nested generated helper over the
payload bytes selected by the enclosing dispatch field. Generated decode-step
helpers and `derive decode` use the same nested helper path. Known dispatch
cases encode by running the nested generated encode helper and then applying
the existing dispatch payload length checks; `derive encode` uses that same
generated encode helper boundary. Unknown `ExtensionDispatch` tags preserve
bounded raw payload bytes and do not attempt nested decoding.

Negative computed lengths report `schema.length_out_of_bounds`, short inputs
keep the ordinary byte-view truncation shape, and encode view-count mismatches
return the existing structured `EncodeError` shape on the nested schema field
path. Nested decode failures preserve the parent dispatch field segment before
the nested schema field path.

Missing, forward, or non-`Int` operands remain outside the nested helper slice
and keep the focused `schema.dispatch_payload` helper-boundary diagnostic with
the nested schema field path and unsupported `ByteView` layout fact.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-subtract-decode/`
  checks same-module closed and extension dispatch decode through a nested
  payload schema with `ByteView(left_length - right_length)`, including the
  generated decode-step helper and derived decode codec boundary.
- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-subtract-encode/`
  checks same-module closed and extension dispatch encode through that nested
  helper, including the derived encode codec boundary.
- `../../../examples/specification/run/binary-schema-imported-dispatch-nested-byteview-subtract-decode/`
  checks the same decode boundary when the nested payload schema is public and
  imported through a written `use` path.
- `../../../examples/specification/run/binary-schema-imported-dispatch-nested-byteview-subtract-encode/`
  checks the same encode boundary when the nested payload schema is public and
  imported through a written `use` path.
- `../../../examples/specification/run/binary-schema-dispatch-nested-byteview-subtract-failure-json/`
  checks that negative nested lengths preserve the parent dispatch field and
  nested schema field path in the structured diagnostic.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
dispatch payload schemas outside the generated helper vocabulary, recursive
payload forms outside the selected length-bounded slices, and mapping behavior
outside the implemented structural slices.
