# Binary Schema Dispatch One-Bit Reserved Payload Helpers

Status: implemented

This record preserves the completed nested dispatch payload helper slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept same-module nested
payload schemas in `Dispatch(...)` when the nested schema starts with
`ReservedBits(1, 0)` followed by `UInt8`. The payload schema routes through the
same generated helper path used for ordinary schema fields: the reserved field
is representation-only, the visible byte decodes as an `Int`, and encode emits
the declared zero reserved bit followed by the visible byte and zero low
padding bits in the shared two-byte big-endian slice.

Nested decode failures keep the parent dispatch field path before the nested
schema field path. Reserved-bit mismatch diagnostics report the absolute byte
offset of the nested payload within the enclosing packet.

Other reserved values or layouts outside the implemented generated-helper
slices continue to report `schema.dispatch_payload` at the parent dispatch
field, with related context naming the nested schema field and helper
boundary.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-reserved-payload-roundtrip/`
  checks successful closed dispatch decode and encode for
  `ReservedBits(1, 0)` followed by `UInt8`.
- `../../../examples/specification/run/binary-schema-dispatch-one-bit-reserved-payload-failure-json/`
  checks the reserved-bit mismatch diagnostic, including the nested field path
  and absolute byte offset.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
  checks that the previous `UnsupportedReservedPayload` shape no longer
  reports `schema.dispatch_payload` solely because of the one-bit reserved
  prefix.
- `../../../examples/specification/check/binary-schema-dispatch-payload-helper-boundary-json/`
  checks that an unsupported adjacent reserved value still reports the helper
  boundary diagnostic.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit payload layouts, dispatch forms, primitive shapes, and mapping
behavior outside the implemented generated-helper slices.
