# Binary Schema Reserved Bit Mapping Exposure

Status: implemented

This record preserves the completed opt-in reserved-bit mapping exposure slice
from `binary-schema-primitives-and-dispatch.md`. Current
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

This historical slice was retired with schema-level mapping support.
Earlier implementation paths kept mapped encode projection eligible for
supported reserved fields and checked the declared reserved value before
emitting bytes.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
field layouts, dispatch forms, primitive shapes, and mapping behavior outside
the implemented generated-helper slices.
