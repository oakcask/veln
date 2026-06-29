# Binary Schema Dispatch Reserved Byte Prefix Payload Helpers

Status: implemented

This record preserves the completed nested dispatch payload helper slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept same-module nested
payload schemas in `Dispatch(...)` when the nested schema uses the supported
reserved-byte-prefix layouts `ReservedBits(2, 0)` followed by `UInt8` or
`ReservedBits(9, 0)` followed by `UInt8`. The payload schema routes through
the same generated helper path used for ordinary schema fields: the reserved
field is representation-only, the visible byte decodes as an `Int`, and encode
emits the declared zero reserved prefix with the visible byte in the same
two-byte big-endian slice.

Nested decode failures keep the parent dispatch field path before the nested
schema field path. Reserved-bit mismatch diagnostics report the absolute byte
offset of the nested payload within the enclosing packet.

Other reserved values or layouts outside the implemented generated-helper
slices continue to report the existing reserved-bit layout or dispatch payload
helper diagnostics.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-reserved-byte-prefix-payload-decode-encode/`
  checks successful closed dispatch decode and encode for
  `ReservedBits(2, 0)` followed by `UInt8` and for
  `ReservedBits(9, 0)` followed by `UInt8`.
- `../../../examples/specification/run/binary-schema-dispatch-reserved-byte-prefix-payload-failure-json/`
  checks the reserved-bit mismatch diagnostic, including the nested field path
  and absolute byte offset.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit payload layouts, dispatch forms, primitive shapes, and mapping
behavior outside the implemented generated-helper slices.
