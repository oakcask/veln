---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Binary Data u56 Byte Helpers

This record preserves the completed source-visible seven-byte unsigned integer
helper slice from the binary data standard library proposal. Current behavior
is specified by `../../specification/names-effects.md`,
`../../specification/names-effects.md`, and
`../../specification/execution.md`.

## Completed Behavior

The prelude exposes ordinary pure source-visible helpers for seven-byte
unsigned integers:

- `byte_read_u56_be(view: ByteView) -> Result<Int, String>`
- `byte_read_u56_le(view: ByteView) -> Result<Int, String>`
- `byte_write_u56_be(value: Int) -> Result<ByteChunk, String>`
- `byte_write_u56_le(value: Int) -> Result<ByteChunk, String>`

The helpers use the same seven-byte big-endian and little-endian
representations as `UInt56be` and `UInt56le`. Reads require a seven-byte view.
Writes accept values in the `0..72057594037927935` range and return
`Err(String)` for unrepresentable values.

## Evidence

- `../../../examples/specification/run/binary-byteview-u56-helpers/` checks
  successful big-endian and little-endian read/write behavior, emitted
  `ByteChunk` values, maximum accepted values, write range failures, and
  truncated read failures.
- `../../../crates/veln-sema/src/tests/prelude_and_callable_values.rs` keeps
  semantic prelude and callable-value coverage for the helper names.
- `../../../crates/veln-backend-jvm/src/tests.rs` keeps JVM runtime method
  mapping coverage for the helper names.
