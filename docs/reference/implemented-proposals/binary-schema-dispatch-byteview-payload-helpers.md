# Binary Schema Dispatch ByteView Payload Helpers

Status: implemented

This record preserves the completed nested dispatch payload helper slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept same-module and public
imported nested payload schemas in `Dispatch(...)` and
`ExtensionDispatch(...)` when the nested schema exposes the generated helper
path and contains a length-bounded `ByteView(length_field)` field whose length
field is an earlier visible `Int` field in that nested schema.

Known dispatch cases decode by running the nested generated helper over the
bounded payload bytes selected by the enclosing dispatch field. Known dispatch
cases encode by running the nested generated encode helper and then applying
the existing dispatch payload length checks. Unknown `ExtensionDispatch` tags
preserve bounded raw payload bytes and do not attempt nested decoding.

Unsupported nested `ByteView` payload layouts whose length field is missing,
forward, not decoded as `Int`, or otherwise outside the generated helper slice
continue to report `schema.dispatch_payload` at the parent dispatch field, with
related context naming the nested payload schema and expected helper boundary.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-byteview-payload-decode/`
  checks same-module closed and extension dispatch decode, including unknown
  extension payload preservation.
- `../../../examples/specification/run/binary-schema-dispatch-byteview-payload-encode/`
  checks same-module closed and extension dispatch encode through the nested
  helper.
- `../../../examples/specification/run/binary-schema-imported-dispatch-byteview-payload-decode/`
  checks imported public payload schema decode through a written `use` path.
- `../../../examples/specification/run/binary-schema-imported-dispatch-byteview-payload-encode/`
  checks imported public payload schema encode through a written `use` path.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
  checks that unsupported `ByteView` payload layouts still report focused
  `schema.dispatch_payload` diagnostics.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`
  checks the human diagnostic route for the same helper eligibility boundary.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
dispatch payload schemas outside the generated helper vocabulary, recursive
payload forms outside the selected length-bounded slices, and mapping behavior
outside the implemented structural slices.
