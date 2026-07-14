# Binary Schema Same-Module Recursive Dispatch Helpers

Status: implemented

This record preserves the completed same-module recursive dispatch payload
helper slice from `binary-schema-primitives-and-dispatch.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/names-effects.md`,
`../../specification/examples.md`, and the checked executable examples under
`../../../examples/specification/`.

## Outcome

Length-bounded same-module `Dispatch(tag_field, length_field, ...)` and
`ExtensionDispatch(tag_field, length_field, ...)` fields may name their own
schema as a recursive payload when the dispatch cases include at least one
non-recursive primitive base case.

The closed helper exposes the finite primitive payload shape. The
extension-tolerant helper wraps that payload type in `SchemaDispatchPayload<T>`
and still preserves unknown tags as bounded raw payload bytes. Decode helpers
collapse recursive known payload chains to the primitive payload value. Encode
helpers accept the same schema-local visible record shape and encode the
primitive base case while keeping dispatch length checks.

Unsupported recursive forms remain on the existing `schema.dispatch_payload`
diagnostic route, including unbounded recursive payload schemas, recursive
payload schemas that do not expose the bounded helper required by the parent
helper, recursive parents without a length field, and recursive parents
without a non-recursive primitive base case.

## Evidence

- `../../../examples/specification/run/binary-schema-recursive-dispatch-decode-encode/`
  checks successful recursive decode collapse and primitive-base encode.
- `../../../examples/specification/run/binary-schema-recursive-dispatch-rejected/`
  checks the missing primitive base diagnostic through `run`.
- `../../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`
  keeps the broader recursive helper rejection route pinned.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
recursive dispatch forms outside this length-bounded primitive-payload helper
boundary and for unsupported field layouts outside the implemented generated
helper vocabulary.
