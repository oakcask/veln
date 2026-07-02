# Binary Schema Dispatch Lowercase One-Bit Reserved Payload

Status: implemented

This record preserves the completed direct dispatch payload slice from
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/`.

## Outcome

Generated binary schema decode and encode helpers accept direct closed and
extension dispatch payload cases written as `uint1 reserves 0`.

The payload is representation-only. Decode validates the high bit of the
payload storage byte, reports `schema.reserved_bits_mismatch` at the payload
byte offset when the bit is not zero, and exposes `()` as the payload value.
Encode writes the fixed zero bit as a one-byte payload. Extension dispatch
length checks continue to compare the supplied length against that one-byte
encoded payload.

The slice is intentionally bounded to the direct `uint1 reserves 0` payload
spelling. Wider non-byte-aligned reserved payloads remain outside this record
until there is a protocol need and a broader direct payload bit-layout model.

## Evidence

- `../../../examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-decode-encode/`
  checks successful closed and extension dispatch decode and encode for
  `uint1 reserves 0`.
- `../../../examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-mismatch-json/`
  checks the reserved-bit mismatch diagnostic, payload byte offset, field
  path, bit width, expected value, actual value, and byte preview.
- `../../../examples/specification/check/lowercase-schema-reserves-diagnostics/`
  no longer rejects `uint1 reserves 0` solely because it appears in a binary
  dispatch payload position.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
reserved-bit payload layouts, dispatch forms, primitive shapes, and behavior
outside the implemented direct one-bit reserved payload slice.
