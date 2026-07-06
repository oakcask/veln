# Binary Schema Dispatch Lowercase Subbyte Reserved Payloads

Status: implemented

This record preserves the completed bounded direct dispatch payload slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept direct closed and
extension dispatch payload cases written as zero-reserved subbyte lowercase
payloads from `uint1 reserves 0` through `uint7 reserves 0`.

The payload is representation-only. Decode consumes one payload storage byte,
validates the declared high-order bits, reports
`schema.reserved_bits_mismatch` at the payload byte offset when those bits are
not zero, and exposes `()` as the payload value. Encode writes the fixed zero
bits as a one-byte payload. Extension dispatch length checks continue to
compare the supplied length against that one-byte encoded payload.

The slice is intentionally bounded to zero-reserved one-byte subbyte payloads.
Byte-aligned reserved dispatch payloads continue to use the existing
byte-aligned path. Nonzero subbyte reserved payloads are covered by
[Binary Schema Dispatch Nonzero Lowercase Subbyte Reserved Payloads](binary-schema-dispatch-nonzero-lowercase-subbyte-reserved-payloads.md).
Non-byte-aligned reserved payloads that span more than one byte remain outside
this record.

## Evidence

- `../../../examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-decode-encode/`
  checks successful closed and extension dispatch decode and encode for
  `uint1 reserves 0`, `uint2 reserves 0`, and `uint7 reserves 0`.
- `../../../examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-mismatch-json/`
  checks the reserved-bit mismatch diagnostic, payload byte offset, field
  path, bit width, expected value, actual value, and byte preview for
  `uint3 reserves 0`.
- `../../../examples/specification/check/lowercase-schema-reserves-diagnostics/`
  no longer rejects supported zero-reserved subbyte payload spellings solely
  because they appear in a binary dispatch payload position, while preserving
  unsupported dispatch payload coverage for a reserved value that exceeds its
  declared width.

## Boundaries Preserved

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit payload layouts, dispatch forms, primitive shapes, and behavior
outside the implemented bounded zero-reserved subbyte payload slice.
