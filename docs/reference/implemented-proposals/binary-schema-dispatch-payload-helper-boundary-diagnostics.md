# Binary Schema Dispatch Payload Helper Boundary Diagnostics

Status: implemented

This record preserves the completed nested dispatch payload helper-boundary
diagnostics slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/diagnostics-json-full.md`,
`../../specification/examples.md`, and the checked executable examples under
`../../../examples/specification/`.

## Outcome

Resolved nested binary dispatch payload schemas that cannot expose the
generated helper required by the parent dispatch helper are rejected with
`schema.dispatch_payload` before helper generation. The structured diagnostic
names the parent schema and field, the parent dispatch field path, the selected
payload schema, the generated decode or encode helper boundary, unavailable
helper directions, and the specific unsupported nested layout reason.

The checked payload shapes include a nested `ByteView(length_field)` layout
whose length source cannot be used by generated decode and encode helpers, a
representation-only reserved-bit layout outside the supported packed helper
forms, and a mapped payload schema whose decode mapping is resolvable but whose
mapping assignment cannot project back to schema-local fields for generated
encode.

Recursive dispatch payload rejections also use `schema.dispatch_payload` with a
focused reason for the failed recursive-helper fact. Checked cases cover a
missing length-bounded parent dispatch, an unmapped decode-only parent that
lacks a primitive base case, and an encode-required imported recursive payload
whose selected mappings do not cover every dispatch case.

## Evidence

- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-boundary-json/`
  pins stable JSON fields for a resolved nested payload schema whose layout
  prevents generated decode and encode helper exposure.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
  checks the broader helper eligibility diagnostic set, including related
  helper-boundary context.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`
  checks the human diagnostic route for the same helper eligibility boundary.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
unsupported field layouts and schema value mapping outside the implemented
helper vocabulary and mapping projection slices.
