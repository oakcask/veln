# Binary Schema General Reserved Byte Prefixes

Status: implemented

This record preserves the completed direct reserved-byte-prefix slice from
`binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated decode and encode helpers accept a direct
`ReservedBits(width, value)` followed by `UInt8` when the width is positive
and non-byte-aligned, the value fits that width, and the group including its
trailing padding fits in the existing at-most-eight-byte big-endian storage
boundary. This replaces the former width-specific helper exception.

The reserved prefix and trailing padding remain representation-only. Decode
exposes only the visible byte and validates the declared reserved value.
Encode accepts only the visible byte, emits the declared prefix and zero
padding, and retains the visible field's existing range diagnostic. Direct and
derived codec paths calculate storage width, consumed bytes, truncation, and
reserved-bit mismatch details from the declared bit width.

## Evidence

- `../../../examples/specification/run/binary-schema-general-reserved-byte-prefix-decode-encode/`
  checks the formerly rejected three-bit nonzero prefix, direct and derived
  decode and encode, consumed-byte accounting, truncation, reserved mismatch,
  visible-byte range failure, and the maximum accepted width boundary.
- `../../../examples/specification/run/binary-schema-general-reserved-byte-prefix-json/`
  checks structured `schema.reserved_bits_mismatch` details for the general
  prefix route.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks
  representative widths around storage boundaries, the maximum accepted
  width, an out-of-range reserved value, and the wider-than-eight-byte
  boundary.

## Boundaries

Dispatch and repeat payload extensions, middle or suffix layouts,
little-endian packing, and groups wider than eight bytes remain outside this
slice. The rule is bounded by the existing 64-bit storage representation, so
future work should not extend it as another width whitelist.
