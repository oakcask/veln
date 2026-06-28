# Binary Schema Reserved Bit Mapping Exposure

Status: implemented

This record preserves the completed opt-in reserved-bit mapping exposure slice
from `../../proposals/binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated binary schema decode helpers keep supported `ReservedBits(width,
value)` fields representation-only by default. The decoded record still omits
the reserved field unless a structural `map to` assignment explicitly names
that schema-local reserved field.

When a mapping assignment names a supported reserved field, decode exposes the
already validated reserved value as an `Int` mapping source. Invalid reserved
bits still report `schema.reserved_bits_mismatch` at the reserved field path
before the structural mapping returns a value.

Generated encode helpers for directly mapped records can project a target
`Int` field back to a supported reserved field. The helper accepts the target
value only when it equals the declared reserved pattern, reports
`codec.encode_mapping_mismatch` at the mapped target field path otherwise, and
still emits the schema-declared reserved bits.

This slice does not make reserved fields visible in generated value records by
default, expose reserved bits to protocol-state logic, add new reserved-bit
layout families, or add arbitrary mapping functions or new mapping syntax.

## Evidence

- `../../../examples/specification/run/binary-schema-byte-aligned-reserved-mapping-decode-encode/`
  checks byte-aligned reserved-field mapping on decode and mapped-record encode,
  including `codec.encode_mapping_mismatch` when the mapped target value differs
  from the declared reserved pattern.
- `../../../examples/specification/run/binary-schema-packed-reserved-mapping-decode-encode/`
  checks the same opt-in mapping behavior for a packed reserved-prefix layout.
- `crates/veln-sema/src/schema/mapping.rs` admits supported reserved fields as
  `Int` mapping sources only for mapping clauses that explicitly name them.
- `crates/veln-sema/src/types.rs` and
  `crates/veln-backend-jvm/src/runtime/collections.java.inc` keep mapped encode
  projection eligible for supported reserved fields and check the declared
  reserved value before emitting bytes.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
field layouts, dispatch forms, primitive shapes, and mapping behavior outside
the implemented generated-helper slices.
