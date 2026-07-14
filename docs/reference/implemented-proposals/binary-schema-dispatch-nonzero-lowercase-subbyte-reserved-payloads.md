# Binary Schema Dispatch Nonzero Lowercase Subbyte Reserved Payloads

Status: implemented

This record preserves the completed bounded direct dispatch payload slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept direct closed and
extension dispatch payload cases written as nonzero subbyte lowercase
reserved-bit payloads from `uint1 reserves 1` through
`uint7 reserves 127`, with each accepted value bounded by its declared width.

The payload is representation-only. Decode consumes one payload storage byte,
validates the declared high-order bits against the reserved value, reports
`schema.reserved_bits_mismatch` at the payload byte offset when those bits do
not match, and exposes `()` as the payload value. Encode writes the declared
reserved bits as a one-byte payload. Extension dispatch length checks continue
to compare the supplied length against that one-byte encoded payload.

The slice is intentionally bounded to direct one-byte subbyte payloads whose
reserved value fits the declared width. Byte-aligned reserved dispatch payloads
continue to use the existing byte-aligned path. Non-byte-aligned reserved
payloads that span more than one byte remain outside this record.

## Evidence

- `../../../examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-decode-encode/`
  checks successful closed and extension dispatch decode and encode for
  `uint1 reserves 1`, `uint2 reserves 3`, and `uint7 reserves 127`.
- `../../../examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-mismatch-json/`
  checks the reserved-bit mismatch diagnostic, payload byte offset, field
  path, bit width, expected value, actual value, and byte preview for
  `uint3 reserves 5`.
- `../../../examples/specification/check/lowercase-schema-reserves-diagnostics/`
  preserves unsupported direct dispatch payload coverage for reserved values
  that exceed their declared width.

## Boundaries Preserved

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit payload layouts, dispatch forms, primitive shapes, and behavior
outside the implemented bounded one-byte subbyte payload slice.
