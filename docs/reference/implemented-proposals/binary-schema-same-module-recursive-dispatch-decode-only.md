# Binary Schema Same-Module Recursive Dispatch Decode-Only

Status: implemented

This record preserves the completed same-module recursive dispatch payload
decode-only parent slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Length-bounded parent `Dispatch(tag_field, length_field, ...)` and
`ExtensionDispatch(tag_field, length_field, ...)` fields may name an earlier
same-module recursive payload schema without selected parent mappings when the
payload schema already exposes the bounded recursive generated decode helper
and the parent includes at least one non-recursive primitive case.

The closed parent exposes the payload schema's recursive mapped payload type.
The extension-tolerant parent wraps that payload type in
`SchemaDispatchPayload<T>` and still preserves unknown tags as bounded raw
payload bytes.

This is a decode-only boundary. Parent decode helpers, generated decode-step
helpers, and `derive decode` may use it, while generated encode helpers and
`derive encode` continue to require the selected recursive mapping boundary
that can project schema-local encode fields.

Unsupported recursive forms remain on the existing `schema.dispatch_payload`
diagnostic route, including unbounded recursive payload schemas and recursive
payload schemas that do not expose the generated decode helper required by the
parent helper.

## Evidence

- `../../../examples/specification/run/binary-schema-same-module-recursive-dispatch-decode/`
  checks successful same-module closed and extension-tolerant recursive decode
  without selected parent mappings, including extension unknown preservation.
- `../../../examples/specification/check/binary-schema-imported-recursive-dispatch-payload-accepted/`
  checks the matching declaration-time acceptance path for imported and
  same-module length-bounded unmapped parents.
- `../../../examples/specification/check/binary-schema-recursive-dispatch-payload-diagnostics/`
  keeps unsupported recursive payload shapes on the existing
  `schema.dispatch_payload` diagnostic route.
- `crates/veln-sema/src/tests/declarations_and_names.rs` includes focused
  same-module and imported unmapped recursive dispatch acceptance coverage.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
recursive dispatch forms outside the length-bounded decode-only helper
boundary and for unsupported field layouts outside the implemented generated
helper vocabulary.
